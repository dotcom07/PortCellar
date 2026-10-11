# Download Korean game files from the publisher

The module-local [download tool](../tools/download_korea.py) implements the
Korean publisher protocol used in the 2026-10-11 investigation. It downloads a
new source directory for managed installation; it does not install the Windows
launcher, run the game, sign in, or update an existing game or prefix.
The Windows `Arknights_KR_Gamelauncher-1.8.1-setup.exe` is not required by this
route or by an already created game installation.

## Download and bind

Requirements: macOS, Python 3.9 or newer, `/usr/bin/curl`, and system `liblzma`
for CRC64/XZ. The parent directory must exist, the destination must not exist,
and the game source must remain outside any managed installation. No Python
packages are required. The observed 41.0.1 payload was about 3.81 GB; allow
space for an additional staged game copy and Wine prefix.

Run from the repository root:

```sh
mkdir -p .portcellar/sources .portcellar/installations
python3 modules/arknights/tools/download_korea.py \
  --destination .portcellar/sources/arknights-korea
cargo run -- game installation create \
  --root .portcellar/installations/arknights-korea --id arknights-korea \
  --module arknights --variant windows-korea --profile wine-dxmt \
  --source-root .portcellar/sources/arknights-korea \
  --wine /path/to/Wine/bin/wine64 --dxmt-root /path/to/DXMT
cargo run -- game installation plan --root .portcellar/installations/arknights-korea
cargo run -- game installation launch \
  --root .portcellar/installations/arknights-korea --timeout-seconds 3600
```

Wine and DXMT are separately supplied, reviewed inputs. Wine must be a regular
file with a sibling wineserver. The reference distribution's symlink needed a
private adapter during the [recorded experiment](2026-10-11-korea-run.md);
the downloader does not prepare or distribute that runtime.

On success, `<destination>/.portcellar-download/receipt.json` records the game
version, publisher launch arguments, and every verified file's size, CRC64/XZ,
and local SHA-256. The manifest is retained beside it. Check the receipt's
`launch_arguments` before binding a later game version: the module's current
launch arguments do not automatically consume publisher parameters. The
observed 41.0.1 publisher parameters were empty.

The destination is a preserved source, not the active game. PortCellar copies
it into installation-owned state. Keep all downloaded files, receipts, prefixes,
logs, and account state private; do not commit them. The example uses the ignored
`.portcellar/` directory.

## Protocol and safety boundaries

The tool asks `https://api-launcher-kr.yo-star.com` for game configuration,
manifest location, and CDN configuration, using the public Korean launcher
1.8.1 request-signing protocol. It does not request account credentials.
The returned game manifest selects individual files on the allowlisted Korean
publisher CDN hosts. HTTPS certificate verification remains enabled; curl user
configuration is disabled and redirects are refused. Unexpected response types,
hosts, paths, sizes, or CRC values fail rather than selecting an arbitrary URL.

Files download with eight workers into retained partial files. Each file must
match its manifest size and CRC64/XZ before activation without replacement.
The tool refuses an existing destination, path traversal, Windows reserved
names, Unicode/case collisions, file/directory conflicts, and downloads inside
managed installations. It bounds JSON responses, file count, individual size,
total size, and request duration. CRC64 is an error-detection checksum, not a
cryptographic signature; publisher identity depends on HTTPS. Local SHA-256
values support subsequent input identification.

Failure leaves the new partial directory for inspection, without a successful
receipt. This tool does not resume or erase failed directories; use a different
new destination for a retry. It does not upgrade existing sources or reconcile
game saves. Game execution and the reviewed compatibility claims remain the
responsibility of the module and managed runtime.

## Provenance and verification

Protocol reference:
[LuMiSxh/Arknights-MacOS-Client at aa3a8ed](https://github.com/LuMiSxh/Arknights-MacOS-Client/tree/aa3a8ed7d76d18c9df9ece55026f143f0b98ba1e),
particularly `LauncherAPI.swift`, `GameRegion.swift`, and `CRC64.swift`.
This Python tool is a first-party implementation of that observed protocol;
it does not copy the reference runtime or game binaries.

The private investigation downloaded and verified the complete 41.0.1 manifest
before the recorded game launch. The public tool is checked with synthetic
payloads and refusal cases, including optimized Python. A bounded live test
queries current publisher metadata and downloads/verifies `Arknights.exe`;
it is separate from a second complete-game download.

```sh
python3 -m unittest discover -s modules/arknights/tools -p 'test_*.py'
python3 -O -m unittest discover -s modules/arknights/tools -p 'test_*.py'
```
