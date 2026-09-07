# SimCity 4 1.1.610 investigation

The initial failure occurred when the Windows 1.1.610.0 build ran under Wine
on Apple Silicon macOS. The region menu opened, but Getting Started Tutorial
produced white rendering and could terminate the game.

The root cause was a C++ virtual-table ABI mismatch. MinGW preserved the
declaration order of overloaded SCGL methods, while the original MSVC-built
game expected a different order. At the affected `TexEnv` call, the game passed
integer mode `1`; the wrong entry dispatched to the color-pointer overload and
eventually passed address `0x1` to `glTexEnvfv`.

The six corrected offsets are:

| Offset | Required entry |
| --- | --- |
| `0x6c` | `TexEnv(..., const float*)` |
| `0x70` | `TexEnv(..., int)` |
| `0xd8` | `TexStageCombine` scale |
| `0xdc` | `TexStageCombine` operand |
| `0xe0` | `TexStageCombine` source |
| `0xe4` | `TexStageCombine` mode |

The aggregate patch includes more than these six entries. It also contains the
1.1.610 mode fallback, Apple legacy WGL context selection, buffer-region state
restoration, capability handling, and optional diagnostics. These changes must
remain attributed to the patch as a whole when reproducing the artifact.

The recorded binary identities are:

| Artifact | SHA-256 |
| --- | --- |
| Original game executable | `8810169790a3ebcbe656205f2d339dfa703c87c4964a51b8417d4a029ebc99bc` |
| Historical corrected SCGL DLL | `f197a05fc36479383b0d1bae3d36e6fe662555d65cf08ed92bf2004ac87a6152` |

The historical corrected run reached the region menu and Getting Started
Tutorial, and the user confirmed that tutorial terrain rendered. That is a
human observation tied to the recorded configuration. It is not an automated
visual assertion and does not establish full compatibility.

The deterministic verifier checks the six vtable entries from a built DLL. It
does not need game files:

```sh
python3 modules/simcity-4/tools/check-scgl-texture-abi.py path/to/SCGL.dll
python3 -O modules/simcity-4/tools/check-scgl-texture-abi.py path/to/SCGL.dll
```

The release path must separately record the SCGL source revision, compiler,
CMake configuration, patch digest, Wine component versions, executable hash,
and the exact launch and scenario observations. Do not turn one successful
scenario into a general compatibility claim.
