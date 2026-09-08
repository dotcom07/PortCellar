# SimCity 4

This module records the first PortCellar compatibility investigation.

**Tested game:** SimCity 4 Deluxe 1.1.610.0 (Windows x86)  
**Tested host:** Apple Silicon macOS  
**Runtime:** Wine Staging 11.10 with a MinGW-built SCGL artifact  
**Status:** Experimental — scenario verified

The scenario results below are historical manual observations of the recorded
configuration, not a fresh verification of a PortCellar package. See the
[investigation](docs/investigation.md) for attribution and outstanding checks.

## Verified

- The game launches.
- The region menu renders.
- Getting Started Tutorial launches.
- Tutorial terrain renders with the corrected SCGL artifact.
- The previously observed texture-related access violation was not reproduced
  in the corrected run.
- The SCGL texture vtable binary check passes for all six declared slots.

## Not yet verified

- Long gameplay sessions.
- Save/load across varied cities.
- Every rendering path or display mode.
- Plugins and DLL mods beyond the tested SCGL path.
- Large developed cities.
- Complete SCGL ABI compatibility.
- Steam edition compatibility.

## For players

The repository does not contain the game, a Steam account, a Wine engine, or
proprietary CrossOver components. The current release material is a source and
reproduction package; it is not a turnkey player download.

Use a legally installed Windows copy that matches the tested game version. Keep
the original installation and saves unchanged while testing. Report results
with the game version, host hardware, macOS version, Wine version, SCGL build,
display mode, scenario, and outcome.

## For developers

Inspect the descriptor and its portable profile with
`cargo run -- game module --root modules --id simcity-4` from the repository root.
This catalog entry does not install SCGL or bind local game/engine paths.

The original game expects a different C++ virtual-table layout from the one
produced by the unmodified MinGW build of SCGL. Six texture-related entries were
reordered for the 1.1.610 ABI. The aggregate SCGL patch also contains the
initialization, OpenGL context, buffer-region, display-mode, and diagnostic
changes documented in [the investigation](docs/investigation.md).

The historical build process used the pinned `upstreams/scgl` revision and the
module patch. A fresh disposable SCGL checkout was built on 2026-09-08 with
the documented i686 MinGW path, and its DLL passed the checker in normal and
optimized Python. Follow the [developer reproduction guide](docs/reproduce.md)
for the commands and the remaining gates before a clean game run can be called
reproducible. A binary check proves only the declared ABI slots; it does not
prove gameplay compatibility.

- [Investigation and evidence](docs/investigation.md)
- [SCGL patch notes](patches/scgl/README.md)
- [SCGL ABI checker](tools/check-scgl-texture-abi.py)
