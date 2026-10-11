import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "download_korea", Path(__file__).with_name("download_korea.py")
)
download = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(download)


class DownloadTests(unittest.TestCase):
    def test_crc64_known_vector_and_streaming(self):
        crc = download.crc64()
        self.assertEqual(crc(b"123456789", 9, 0), 0x995DC9BBDF1939FA)
        self.assertEqual(crc(b"", 0, 0), 0)
        self.assertEqual(crc(b"56789", 5, crc(b"1234", 4, 0)), 0x995DC9BBDF1939FA)

    def test_manifest_rejects_escaping_colliding_and_reserved_paths(self):
        for paths in (["/../outside"], ["/C:/outside"], ["/a\\b"], ["//a"],
                      ["/CON.txt"], ["/a."], ["/a", "/A"], ["/a", "/a/b"],
                      ["/.portcellar-download/receipt.json"], ["/a", "/Arknights.exe/"]):
            files = [{"path": p, "size": "0", "hash": "0"} for p in paths]
            with self.subTest(paths=paths), self.assertRaises(ValueError):
                download.validate_manifest({"source": "/game", "file": files})

    def test_manifest_rejects_invalid_sizes_and_checksums(self):
        for size, checksum in (("-1", "0"), ("4294967297", "0"), ("1", "-1"),
                               ("1", str(2**64)), ("1", "abc"), (True, "0")):
            with self.subTest(size=size, checksum=checksum), self.assertRaises(ValueError):
                download.validate_manifest({"source": "/game", "file": [
                    {"path": "/Arknights.exe", "size": size, "hash": checksum}]})

    def test_urls_reject_foreign_origins_and_credentials(self):
        download.validate_url("https://launcher-pkg-ark-kr.yo-star.com/game/file")
        for url in ("http://launcher-pkg-ark-kr.yo-star.com/game", "https://evil.test/file",
                    "https://user@launcher-pkg-ark-kr.yo-star.com/file",
                    "https://launcher-pkg-ark-kr.yo-star.com:444/file",
                    "https://launcher-pkg-ark-kr.yo-star.com/file#fragment"):
            with self.subTest(url=url), self.assertRaises(ValueError):
                download.validate_url(url)

    def test_transport_refuses_redirects_and_authorization_to_cdn(self):
        with patch.object(download.subprocess, "run") as run:
            with self.assertRaises(ValueError):
                download.fetch("https://launcher-pkg-ark-kr.yo-star.com/file", 100,
                               authorization="protocol-signature")
            run.assert_not_called()
            run.return_value.stderr = b"302"
            with self.assertRaises(ValueError):
                download.fetch("https://launcher-pkg-ark-kr.yo-star.com/file", 100)

    def test_managed_destination_and_unicode_collisions_are_refused(self):
        with tempfile.TemporaryDirectory() as name:
            parent = Path(name)
            (parent / ".portcellar-installation.toml").write_text("owned")
            with patch.object(download, "publisher_metadata") as fetch:
                with self.assertRaises(ValueError):
                    download.download_game(parent / "game")
                fetch.assert_not_called()
        for paths, size in ((["/Arknights.exe", "/caf\u00e9", "/cafe\u0301"], "0"),
                            (["/Arknights.exe", "/b", "/c"], str(4 * 1024**3))):
            files = [{"path": p, "size": size, "hash": "0"} for p in paths]
            with self.subTest(paths=paths), self.assertRaises(ValueError):
                download.validate_manifest({"source": "/game", "file": files})

    def test_existing_output_is_preserved_without_network(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name) / "game"
            root.mkdir()
            sentinel = root / "save.dat"
            sentinel.write_bytes(b"save")
            with patch.object(download, "publisher_metadata") as fetch:
                with self.assertRaises(FileExistsError):
                    download.download_game(root)
                fetch.assert_not_called()
            self.assertEqual(sentinel.read_bytes(), b"save")

    def test_verified_download_and_corrupt_payload_retention(self):
        payload = b"123456789"
        manifest = {"source": "/game", "file": [{
            "path": "/Arknights.exe", "size": "9", "hash": str(0x995DC9BBDF1939FA)}]}
        metadata = {"version": "test", "launch_arguments": []}

        def fetch(url, limit, output=None, authorization=None):
            output.write(payload)

        with tempfile.TemporaryDirectory() as name:
            for valid in (True, False):
                root = Path(name) / str(valid)
                with patch.object(download, "publisher_metadata", return_value=(
                    metadata, manifest, "https://launcher-pkg-ark-kr.yo-star.com")), \
                        patch.object(download, "fetch", side_effect=fetch):
                    if valid:
                        receipt = download.download_game(root)
                        self.assertEqual(receipt["bytes"], 9)
                        self.assertEqual((root / "Arknights.exe").read_bytes(), payload)
                        self.assertEqual(len(receipt["files"][0]["sha256"]), 64)
                        self.assertTrue((root / ".portcellar-download/receipt.json").is_file())
                    else:
                        manifest["file"][0]["hash"] = "0"
                        with self.assertRaises(ValueError):
                            download.download_game(root)
                        self.assertFalse((root / "Arknights.exe").exists())
                        self.assertFalse((root / ".portcellar-download/receipt.json").exists())
                        self.assertEqual((root / ".portcellar-download/0.part").read_bytes(), payload)


if __name__ == "__main__":
    unittest.main()
