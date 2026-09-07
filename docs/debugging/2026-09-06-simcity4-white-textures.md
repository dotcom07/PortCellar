# SimCity 4: white textures and tutorial failure

Date: 2026-09-06. Game: 1.1.610.0, PE32/i386. Runtime: Wine Staging
11.10 on Apple M4 Pro, macOS 26.6.2, with the MinGW-built SCGL driver.

## Symptom

The region menu opened, but selecting **Getting Started Tutorial** produced
white areas and could terminate the game. Surviving the launch smoke test did
not establish that city rendering worked.

## Root cause

SCGL's C++ virtual-function table did not match the game's binary ABI.
MinGW retained declaration order for overloaded methods, while the original
MSVC-built game expected a different order.

At slot `0x70`, the game passed texture mode `1`. SCGL dispatched that call to
the overload accepting a color-array pointer, forwarding address `0x1` to
`glTexEnvfv`. This was an argument-type mismatch, not missing game data or an
insufficient-memory diagnosis.

The four `TexStageCombine` overloads also occupied the wrong slots, corrupting
the texture-combining settings needed for terrain rendering.

| Slot | Required overload |
| --- | --- |
| `0x6c` | `TexEnv(..., const float*)` — color |
| `0x70` | `TexEnv(..., int)` — mode |
| `0xd8` | `TexStageCombine` — scale |
| `0xdc` | `TexStageCombine` — operand |
| `0xe0` | `TexStageCombine` — source |
| `0xe4` | `TexStageCombine` — mode |

## Evidence

- Wine loaded the staged SCGL DLL and the executable at its required
  `0x00400000` base. The original launch already had ASCII paths and LAA.
- Wine repeatedly reported `c0000005` while rendering.
- LLDB stopped in macOS `GLEngine!glTexEnvfv_Exec + 966` at
  `movss (%rcx), %xmm0`, with `EXC_BAD_ACCESS`, address `0x1`.
- Game call sites `0x7a9bef` and `0x7d4816` established scalar/color slots.
  The original OpenGL driver's table at `0xac3688` independently confirmed
  these slots and the combiner order.
- A filtered kernel-log query found no GPU reset/restart message in the
  inspected window. It included sandbox analytics and CPU-wakeup warnings;
  these were not evidence of the texture failure. The captured fault was in
  user-space OpenGL code; no kernel change was needed.

## Fix and verification

Reordered the six overload slots for the existing MinGW 1.1.610 build in
`cIGZGDriver.h`, updated the reproducible SCGL patch, rebuilt the Release DLL,
and installed it into the staged game's `Plugins` directory.

```sh
python3 modules/simcity-4/tools/check-scgl-texture-abi.py .portcellar/build/scgl-release-no-diag/SCGL.dll
```

The binary-level regression check failed against the old DLL and passed against
the rebuilt DLL. The corrected Release run stopped producing the previous
access violations, and the user confirmed that Getting Started Tutorial worked.
Computer Use could not attach to Wine's window, so visual confirmation came
from the user, not an automated screenshot.

A separate diagnostics-enabled run failed earlier during initialization with
a nested signal-stack exception. That run is not counted as a successful test.
Other overload groups (vertex-format and fog) showed additional static ABI
differences; they were left unchanged after the successful tutorial test and
the user's request to proceed only with Korean localization. This is not a
claim of complete game or ABI coverage.

Raw traces, LLDB output, the original DLL, and build logs remain local under
`.portcellar/diagnostics/simcity4-20260906/` and are not committed.
