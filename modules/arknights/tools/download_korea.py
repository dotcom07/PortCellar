#!/usr/bin/env python3
"""Download a new Korean game source from Yostar; never launch or update it.

Protocol reference: LuMiSxh/Arknights-MacOS-Client, revision aa3a8ed7d76d18c9df9ece55026f143f0b98ba1e.
"""

import argparse
import concurrent.futures
import ctypes
import ctypes.util
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import threading
import time
import unicodedata
from urllib.parse import quote, urlencode, urlsplit


API = "https://api-launcher-kr.yo-star.com"
CDN_HOSTS = {"launcher-pkg-ark-kr.yo-star.com", "launcher-pkg-ark-kr-bk.yo-star.com"}
STATE = ".portcellar-download"
MAX_JSON = 16 * 1024**2
MAX_TOTAL = 8 * 1024**3


def validate_url(url):
    if not isinstance(url, str):
        raise ValueError("Expected a publisher URL string")
    parsed = urlsplit(url)
    if (parsed.scheme != "https" or parsed.hostname not in CDN_HOSTS | {urlsplit(API).hostname}
            or parsed.username is not None or parsed.password is not None
            or parsed.port not in (None, 443) or parsed.fragment):
        raise ValueError("Expected a credential-free HTTPS publisher URL")
    return parsed


def fetch(url, limit, output=None, authorization=None):
    parsed = validate_url(url)
    if authorization is not None and parsed.netloc != urlsplit(API).netloc:
        raise ValueError("Authorization is restricted to the launcher API")
    args = ["/usr/bin/curl", "--disable", "--fail", "--silent", "--show-error", "--proto", "=https",
            "--connect-timeout", "15", "--max-time", "600", "--max-filesize",
            str(max(1, limit)), "--write-out", "%{stderr}%{http_code}"]
    if authorization is not None:
        args.extend(["--header", "Authorization: " + authorization])
    # Redirects are deliberately refused; no publisher response selects a new origin.
    result = subprocess.run(args + [url], stdout=output or subprocess.PIPE,
                            stderr=subprocess.PIPE, check=True)
    if result.stderr != b"200":
        raise ValueError("Publisher request did not return HTTP 200")
    if output is None:
        if len(result.stdout) > limit:
            raise ValueError("Publisher response exceeded its size limit")
        return json.loads(result.stdout)


def publisher_metadata():
    head = json.dumps({"game_tag": "Arknights_KR", "time": int(time.time()),
                       "version": "1.8.1"}, separators=(",", ":"))
    # Public launcher protocol constant, not an account credential.
    signature = hashlib.md5((head + "DE7108E9B2842FD460F4777702727869").encode()).hexdigest()
    authorization = '{"head":' + head + ',"sign":"' + signature + '"}'

    def api(path):
        response = fetch(API + path, MAX_JSON, authorization=authorization)
        if response.get("code") != 200 or not isinstance(response.get("data"), dict):
            raise ValueError("Publisher API rejected the request")
        return response["data"]

    config = api("/api/launcher/game/config")
    version = config["game_latest_version"]
    file_path = config["game_latest_file_path"]
    arguments = config["game_start_params"]
    if (not isinstance(version, str) or not isinstance(file_path, str)
            or config["game_start_exe_name"] != "Arknights.exe"
            or not isinstance(arguments, list)
            or not all(isinstance(a, str) and "\0" not in a for a in arguments)):
        raise ValueError("Unexpected Korean game configuration")
    location = api("/api/launcher/game/config/json?" + urlencode(
        {"version": version, "file_path": file_path}))
    manifest_url = location["url"]
    if validate_url(manifest_url).hostname not in CDN_HOSTS:
        raise ValueError("Manifest must be on the publisher CDN")
    manifest = fetch(manifest_url, MAX_JSON)
    cdn = api("/api/launcher/advanced/game/download/cdn")["primary_cdn"]
    parsed = validate_url(cdn)
    if parsed.hostname not in CDN_HOSTS or parsed.path not in ("", "/") or parsed.query:
        raise ValueError("Unexpected publisher CDN base")
    return {"version": version, "launch_arguments": arguments}, manifest, cdn.rstrip("/")


def relative_path(raw):
    if not isinstance(raw, str):
        raise ValueError("Expected a manifest path string")
    value = raw[1:] if raw.startswith("/") else raw
    for part in value.split("/"):
        if (not part or part in (".", "..") or part.endswith((".", " "))
                or any(c in part for c in '\\:\x00<>"|?*') or any(ord(c) < 32 for c in part)
                or re.fullmatch(r"CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9]", part.split(".")[0], re.I)):
            raise ValueError("Unsafe manifest path")
    return value


def path_key(path):
    return unicodedata.normalize("NFC", path).casefold()


def validate_manifest(manifest):
    source = relative_path(manifest["source"])
    entries = manifest["file"]
    if not isinstance(entries, list) or not 0 < len(entries) <= 10000:
        raise ValueError("Invalid manifest file count")
    files = []
    for entry in entries:
        path = relative_path(entry["path"])
        size, checksum = entry["size"], entry["hash"]
        if (isinstance(size, bool) or not isinstance(size, (str, int))
                or not re.fullmatch(r"[0-9]+", str(size)) or int(size) > 4 * 1024**3
                or not isinstance(checksum, str) or not re.fullmatch(r"[0-9]+", checksum)
                or int(checksum) >= 2**64):
            raise ValueError("Invalid manifest size or CRC64")
        files.append({"path": path, "size": int(size), "crc64_xz": str(int(checksum))})
    keys = {path_key(f["path"]) for f in files}
    if len(keys) != len(files) or sum(f["size"] for f in files) > MAX_TOTAL:
        raise ValueError("Duplicate manifest paths or excessive download size")
    for file in files:
        parts = file["path"].split("/")
        if path_key(parts[0]) == STATE or any(
                path_key("/".join(parts[:i])) in keys for i in range(1, len(parts))):
            raise ValueError("Conflicting manifest paths")
    if "Arknights.exe" not in {f["path"] for f in files}:
        raise ValueError("Korean game executable is missing from manifest")
    return source, files


def crc64():
    library = ctypes.util.find_library("lzma")
    if library is None:
        raise RuntimeError("liblzma is required for CRC64/XZ verification")
    function = ctypes.CDLL(library).lzma_crc64
    function.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint64]
    function.restype = ctypes.c_uint64
    if function(b"123456789", 9, 0) != 0x995DC9BBDF1939FA:
        raise RuntimeError("CRC64/XZ known-vector check failed")
    return function


def download_game(destination):
    destination = Path(destination).expanduser().absolute()
    if destination.exists() or destination.is_symlink():
        raise FileExistsError("Destination must be a new directory; existing files are preserved")
    parent = destination.parent.resolve(strict=True)
    for ancestor in [parent, *parent.parents]:
        if (ancestor / ".portcellar-installation.toml").exists():
            raise ValueError("Do not download inside a managed installation")
    destination = parent / destination.name
    metadata, manifest, cdn = publisher_metadata()
    source, files = validate_manifest(manifest)
    checksum = crc64()
    total = sum(f["size"] for f in files)
    if shutil.disk_usage(parent).free < total + 64 * 1024**2:
        raise ValueError("Insufficient disk space for the manifest and retained partial files")
    destination.mkdir()
    state = destination / STATE
    state.mkdir()
    (state / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    lock = threading.Lock()
    completed = 0

    def download(item):
        nonlocal completed
        index, entry = item
        target = destination / entry["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        current = target.parent
        while current != destination:
            if current.is_symlink():
                raise ValueError("Unexpected symlink in download destination")
            current = current.parent
        partial = state / (str(index) + ".part")
        url = cdn + "/" + quote(source + "/" + entry["path"], safe="/")
        with partial.open("xb") as output:
            fetch(url, entry["size"], output=output)
        crc = 0
        sha = hashlib.sha256()
        with partial.open("rb") as data:
            while block := data.read(1024**2):
                crc = checksum(block, len(block), crc)
                sha.update(block)
        if partial.stat().st_size != entry["size"] or str(crc) != entry["crc64_xz"]:
            raise ValueError("Downloaded file failed manifest verification: " + entry["path"])
        # Atomic activation without replacing a concurrently created destination.
        os.link(partial, target, follow_symlinks=False)
        partial.unlink()
        with lock:
            completed += 1
            if completed % 100 == 0 or completed == len(files):
                print(f"Verified {completed}/{len(files)} files", flush=True)
        return {**entry, "sha256": sha.hexdigest()}

    pool = concurrent.futures.ThreadPoolExecutor(max_workers=8)
    futures = [pool.submit(download, item) for item in enumerate(files)]
    try:
        results = [future.result() for future in concurrent.futures.as_completed(futures)]
    finally:
        pool.shutdown(wait=True, cancel_futures=True)
    receipt = {**metadata, "source": source, "bytes": total,
               "method": "Publisher HTTPS manifest, file sizes and CRC64/XZ; local SHA256 inventory",
               "files": sorted(results, key=lambda f: f["path"])}
    (state / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", required=True, type=Path,
                        help="New source directory under an existing parent (never an existing game)")
    args = parser.parse_args()
    try:
        receipt = download_game(args.destination)
    except (ValueError, RuntimeError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Download failed; any partial directory is retained: {error}\n")
    print(f"Verified Korean game {receipt['version']}: {len(receipt['files'])} files, {receipt['bytes']} bytes")


if __name__ == "__main__":
    main()
