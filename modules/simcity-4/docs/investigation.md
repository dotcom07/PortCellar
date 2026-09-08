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

## Source and license review

On 2026-09-07, the aggregate patch passed `git apply --check` against a clean
archive of SCGL commit `dc80faec59980da7436e792171e3ce55778f41cd`. The check
used committed source only, excluding the original worktree's local changes.
This proves source applicability, not build success or gameplay behavior.

Patch SHA-256:
`65060b6c374d1ef5550e4782145746ad327738d0500eab7412fc61b74b6b7721`.

The patch changes 16 source/build files. The six texture entries are only one
part: initialization, renderer capabilities, texture state, viewport/mode
handling, buffer regions, and optional diagnostics also change. The isolated
ABI correction has not been freshly tested as a standalone patch. Do not
describe the aggregate as a six-line fix or use its result to certify the
remaining SCGL ABI.

Upstream source headers identify SCGL as LGPL-2.1-or-later, copyright (C) 2025
Nelson Gomez (nsgomez). The original
[license text](../patches/scgl/LICENSE) is retained unchanged; its SHA-256 is
`415aa7cef0e664ae7ce0dd55300c09916449da57601b1053c63644f648bcfd90`.
Preserve upstream copyright headers and identify PortCellar modifications and
their dates in any distributed modified source. The repository's dual license
does not relicense SCGL-derived code.

Before shipping a DLL, include the applicable license/notices and complete
corresponding modified source, build scripts, dependency versions and license
material, and satisfy any relinking obligations introduced by linked libraries.
Keep proprietary game files out of that source bundle. A source URL alone is
not a complete binary distribution compliance record.

On 2026-09-08, a clean disposable checkout at the pinned revision was patched
and built with CMake 4.1.1, Ninja, and GCC 16.1.0 using the i686 MinGW
compilers. The output was a PE32 DLL, and the imported checker passed against it
under both normal and optimized Python. This is fresh source/build evidence;
the output hash is environment-specific and no new game session was run. A
separate unpatched checkout produced a PE32 DLL that failed the same checker at
slot `0x6c` under both Python modes.

Still needed: fresh launch/region/tutorial observations for the exact output.
The unpatched and patched DLLs were disposable local artifacts, not release
assets. Historical binary hashes do not prove this patch or reproduce its bytes.

## Follow-up: fresh public-source game run

Later on 2026-09-08, a separate build from public commit
`183144c2773e1ee47f80d561da524d54789578a6` reached the region menu and Getting
Started Tutorial after setting `WINE_D3D_CONFIG=renderer=vulkan`. SCGL continued
to use OpenGL for game rendering. The agent captured both screens and the user
confirmed the displayed run worked. This supersedes the earlier statement
that no fresh game session had been run, without extending the claim to long
sessions, save/load, mods or all graphical behavior.

A standalone DirectDraw probe failed during initialization as 32-bit code with
the installed Wine 11.10 engine's default OpenGL backend. The same probe passed
as 64-bit code, and as 32-bit code with the Vulkan backend. No game or SCGL DLL
was needed to reproduce the failure. This identifies a separate Wine startup
problem; it does not change the texture-vtable ABI finding above. See the
[reproduction guide](reproduce.md#fresh-game-run-observation-2026-09-08) for
build identity, commands, observed limits and the minimal probe.
