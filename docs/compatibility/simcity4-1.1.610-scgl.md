# SimCity 4 1.1.610 SCGL compatibility

This document is the public compatibility summary for the SimCity 4 module.
The detailed investigation is maintained at
[`modules/simcity-4/docs/investigation.md`](../../modules/simcity-4/docs/investigation.md).

## Scope

- Game: SimCity 4 Deluxe 1.1.610.0, Windows PE32/i386
- Host: Apple Silicon macOS
- Runtime: Wine Staging 11.10
- Graphics path: MinGW-built SCGL with the 1.1.610 ABI patch
- Historical observed scenarios: launch, region menu, and Getting Started Tutorial

## Finding

The original MSVC-built game and the unmodified MinGW SCGL build used different
virtual-table layouts for overloaded texture methods. At one affected call, the
game passed integer mode `1`, but the wrong SCGL slot selected the color-pointer
overload and eventually passed address `0x1` to `glTexEnvfv`.

The module patch reorders six texture-related entries:

| Offset | Entry |
| --- | --- |
| `0x6c` | `TexEnv(..., const float*)` |
| `0x70` | `TexEnv(..., int)` |
| `0xd8` | `TexStageCombine` scale |
| `0xdc` | `TexStageCombine` operand |
| `0xe0` | `TexStageCombine` source |
| `0xe4` | `TexStageCombine` mode |

The aggregate patch also contains the 1.1.610 mode fallback, Apple legacy WGL
context selection, buffer-region state restoration, capability handling, and
optional diagnostics. The six-slot description does not describe the complete
patch by itself.

## Reproduction

Build from the pinned SCGL source and apply
[`simcity4-1.1.610-abi.patch`](../../modules/simcity-4/patches/scgl/simcity4-1.1.610-abi.patch).
Keep release and diagnostic builds separate. The complete build and checker
procedure is in the [developer reproduction guide](../../modules/simcity-4/docs/reproduce.md).
The binary verifier needs no game files:

```sh
python3 modules/simcity-4/tools/check-scgl-texture-abi.py path/to/SCGL.dll
python3 -O modules/simcity-4/tools/check-scgl-texture-abi.py path/to/SCGL.dll
```

A passing verifier proves the six declared binary slots only. The historical
corrected run reached the tutorial and the user confirmed that terrain rendered;
that is a human observation tied to one runtime configuration. A fresh game
rerun for the current source build has not been recorded.

## Limits

This record does not establish full SimCity 4 compatibility. Long sessions,
save/load behavior, all display modes, all rendering paths, plugins and DLL
mods, Steam edition behavior, and the remainder of the SCGL ABI require separate
scenario evidence.
