# Arknights

Status: experimental Korean Windows profiles. On 2026-10-11, the user confirmed
a visible game window and a subsequent 1280 x 720 windowed launch through
PortCellar. The official installer did not complete in the bounded experiments;
the game files were obtained separately from the publisher's CDN. Launcher UI,
login, and gameplay remain unverified. See the
[run observations](docs/2026-10-11-korea-run.md) for inputs and limits.

## Reference and applicability

The reference is
[LuMiSxh/Arknights-MacOS-Client at aa3a8ed](https://github.com/LuMiSxh/Arknights-MacOS-Client/tree/aa3a8ed7d76d18c9df9ece55026f143f0b98ba1e).
Its README lists Global, Japan, and Korea as default regions. `GameRegion.swift`
selects the Yostar Korean endpoint `https://api-launcher-kr.yo-star.com`, and
`LauncherAPI.swift` uses launcher API version `1.8.1`.
That is a source observation, not validation of a particular installer binary.

The reference replaces the Windows launcher with its own native downloader and
launches `Arknights.exe`. It therefore does not by itself prove that
`Arknights_KR_Gamelauncher-1.8.1-setup.exe` or the Windows GameLauncher works.
Inspect installer, official launcher, and game executable as separate targets.
Publisher-provided launch parameters may also be required; the candidate does
not query the launcher API or invent those parameters.

The reference `runtime.json` pins Arknights macOS Runtime `0.6.1`, Wine `11.17`,
and DXMT `0.80-244-g7c8dee1`. Its Wine source is `dappermint/winecx` at
`e0aa380780b73e20fabcfe78fd42713b94929a53`. The audio-follow-output setting
is specific to that runtime. The MSYNC setting also requires a supporting Wine
build. Neither becomes a PortCellar global default. Obtain a compatible engine
and inspect its provenance/dependencies before a real experiment; the module
does not download or distribute it.

## Local execution

For a new Korean game source, the module's
[publisher download tool](docs/publisher-download.md) queries the official
API and downloads verified game files from the official CDN. It does not need
the Windows launcher installer. The tool is separate from the Rust runtime;
it does not execute Wine or the downloaded game.

See the [managed standalone guide](../../docs/runtime/managed-installations.md)
for commands and preservation rules. For an existing Korean game directory:

```sh
mkdir -p .portcellar/installations
cargo run -- game installation create \
  --root .portcellar/installations/arknights-kr \
  --id arknights-kr --module arknights --variant windows-korea \
  --profile wine-dxmt --source-root /path/to/installed/Korean/game \
  --wine /path/to/Wine/bin/wine64 --dxmt-root /path/to/DXMT
cargo run -- game installation plan --root .portcellar/installations/arknights-kr
```

Use `installer-baseline` without `--dxmt-root` for a separate installer
experiment. `--source-root` is still required and must be a real directory;
an empty input directory is enough for an installer-only binding. The installer
writes into the new prefix. Its output is not automatically rebound as the
game source. Locate the installed game afterwards and explicitly create a game
binding with that directory. Preserve the installer experiment and its logs.
Do not point an installer at the preserved input game directory.

The DXMT candidate copies architecture-selected payload DLLs beside the staged
game and uses the referenced load-order policy. The upstream client instead
installs both architectures into its prefix. This different placement is a
PortCellar candidate that needs a real launch comparison before any compatibility
claim. The selected Wine must provide its `winemetal` integration.

## Game-specific helpers

[Compatibility notes](docs/compatibility.md) describe the referenced Vuplex,
AppContainer, and PlatformProcess helpers, their fixed paths and prerequisites.
They are not built or applied automatically by this module. Core file-change
operations provide SHA-256 checks, retained originals and replacements,
conditional restoration, and ownership; they do not determine that a helper
is suitable for this game build.

## First verification scenarios

Record executable hashes, installer source/build, Wine and DXMT inputs, host
version/architecture, profile bytes, and applied helper changes. Check these
independently: installer completion, official launcher/update UI, game rendering,
login/browser flow, notices, audio/video, a playable stage, and restart/update
behavior. Mark visual observations explicitly. Keep account state, raw logs,
game files, prefixes, and screenshots in private ignored state.
