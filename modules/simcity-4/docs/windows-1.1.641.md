# GOG 1.1.641: SCGL video-mode vtable mismatch

A controlled comparison on 2026-09-09 isolated an initialization failure to
two `GetVideoModeInfo` entries in a MinGW-built SCGL DLL. GOG 1.1.641.0 calls
the indexed overload through byte offset `0x104`. Placing the current-mode
overload there changes both the first argument's meaning and stack cleanup.

This follows the [1.1.610 investigation](investigation.md), testing the
community-supported GOG version.

## Static evidence

Original executable SHA-256:
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
LAA-staged executable SHA-256:
`f3c42faf44c692dcc4475b9a009f5ef139b855434c60a7befe400a0a750204a7`.
Addresses below are preferred-image virtual addresses.

The mode-enumeration loop calls slot `0xfc` (CountVideoModes) at
`0x98e488`, then calls the same driver through slot `0x104`:

```asm
98e4a0: mov  ecx, [esi+0x9c]  ; driver this pointer
98e4a6: mov  eax, [ecx]       ; driver vtable
98e4a8: lea  edx, [esp+0x10]  ; output sGDMode
98e4ac: push edx
98e4ad: push edi             ; mode index
98e4ae: call [eax+0x104]
```

The caller reads output fields, increments EDI, calls CountVideoModes again
at `0x98e4f6`, and repeats while the index is below the count.
Other indexed-query call sites include `0x98e22f`, `0x98e39b`, and `0x98e516`.

Actual compiled diagnostic DLL mappings:

| Byte offset | Aggregate ABI option OFF | Corrected layout |
| --- | --- | --- |
| `0x100` | `GetVideoModeInfo(uint32_t, sGDMode&)` | `GetVideoModeInfo(sGDMode&)` |
| `0x104` | `GetVideoModeInfo(sGDMode&)` | `GetVideoModeInfo(uint32_t, sGDMode&)` |

The alternatives are declared in `scgl/cIGZGDriver.h` and `scgl/cGDriver.h`
under `SCGL_SIMCITY4_610_ABI`. Its name predates this experiment.
For this MinGW build, the virtual overload declaration order determines these
entries. At runtime the game invokes the pointer at the offset; it does not
resolve a C++ overload by name.

ECX carries `this`; explicit arguments are on the x86 stack. Disassembly of
the option-OFF DLL shows `ret 8` in the indexed method and `ret 4` in the
current-mode method, including their invalid-argument return paths.
Wrong dispatch therefore interprets the index as the output pointer, leaves
the actual output unwritten when the diagnostic pointer guard rejects it,
and removes four argument bytes instead of eight. Each such call leaves the
stack four bytes below its expected position. A pointer guard inside the
wrong overload cannot repair the calling convention mismatch.

## Controlled runtime comparison

Apple M4 Pro, macOS 26.6.2, Wine Staging 11.10, Windows GOG 1.1.641.0,
LAA-staged EXE, windowed SCGL OpenGL rendering at 1600x900.
`WINE_D3D_CONFIG=renderer=vulkan` was held constant for the separate
[DirectDraw initialization workaround](reproduce.md#isolate-the-initialization-failure-without-game-files).
SCGL continued to render through OpenGL.

The experimental DLL was derived from the corrected diagnostic DLL by
exchanging only the four-byte pointers at `0x100` and `0x104`. Byte comparison
found exactly four changed bytes, all within those pointers. Function bodies,
texture slots, diagnostics, mode-count cap and injected modes were identical.
Both runs reported 56 modes.

| Variant | Result |
| --- | --- |
| Corrected control | Valid output pointers; tutorial terrain and Overview UI rendered; manually inspected screenshot; stopped at the 55-second bound. |
| Only mode slots reversed | Indices received as pointers; exit status 255; access violation at stack-region address `0x0022fc12`. |

DLL SHA-256:

- Control: `e6ab24053ae00feb0bebdc3b79ee575c32c6e729abc4797a9d8ee207bd595209`.
- Mode-reversed: `c607a340c22fb1f455f55d4d934e5032d9525467a06d59fa9318b06a4610732e`.

The reversed variant ran first, then the control, using the same staged game
path, prefix, arguments and test user directory, with Wine stopped between
runs. This was sequential, not a prefix-snapshot reset. The user directory
retained earlier tutorial state; the control screenshot showed Getting
Started Tutorial's Overview page.

Reversed variant:

```text
CountVideoModes this=0140bba0=56
GetVideoModeInfo current=-1 target=00000000
GetVideoModeInfo ignored invalid target=00000000
GetVideoModeInfo current=-1 target=00000001
GetVideoModeInfo ignored invalid target=00000001
```

Control:

```text
CountVideoModes this=0140bba0=56
GetVideoModeInfo610 index=0 target=0022fca8 count=56
GetVideoModeInfo610 index=1 target=0022fca8 count=56
```

The `610` suffix is a historical diagnostic label.
The control Wine log had no `code=c0000005` entries in this bounded run.
Wrong dispatch is directly observed; stack imbalance follows from the caller
and callee instructions. The final exception address alone does not reconstruct
every instruction between the bad call and the crash.

An earlier non-diagnostic corrected DLL reached the region menu and tutorial;
Continue advanced to Overview. Its SHA-256 was
`0f87af25d62ed66da276ba6486517d336ecb15cc6501e955a77dbcd327629de8`.
Coastline/water appearance remains incorrect or unresolved. These observations
do not establish long-session stability, save/load or other DLL-mod behavior.

## Reproduce the binary check

Build using the [developer guide](reproduce.md), with
`SCGL_DIAGNOSTICS=ON`, `SCGL_SIMCITY4_610_ABI=ON` and
`SCGL_STATIC_RUNTIME=ON`. The existing aggregate patch contains the
corrected mode declarations.

From the repository root, use the existing PE/symbol reader:

```sh
python3 - path/to/SCGL.dll <<'PY'
import runpy
import sys
from pathlib import Path

check = runpy.run_path('modules/simcity-4/tools/check-scgl-texture-abi.py')
dll = Path(sys.argv[1])
symbols = check['read_symbols'](dll)
data = dll.read_bytes()
table = check['vtable_file_offset'](data, symbols)
expected = {
    0x100: 'nSCGL::cGDriver::GetVideoModeInfo(sGDMode&)',
    0x104: 'nSCGL::cGDriver::GetVideoModeInfo(unsigned int, sGDMode&)',
}
for slot, name in expected.items():
    actual = check['read_u32'](data, table + slot, 'mode slot')
    if actual != symbols[name]:
        raise SystemExit(f'FAIL: slot {slot:#x} does not point to {name}')
    print(f'PASS: {slot:#x} -> {name}')
PY
```

Requires `i686-w64-mingw32-nm` on PATH. This check passes the control and
fails the mode-reversed variant. The six-slot texture checker passes both:
all six texture entries were preserved.

Inspect the caller and DLL return instructions independently:

```sh
i686-w64-mingw32-objdump -d --start-address=0x98e47e \
  --stop-address=0x98e51c 'path/to/SimCity 4.exe'
i686-w64-mingw32-objdump -d -C path/to/SCGL.dll > scgl-disassembly.txt
```

## Runtime prerequisites

Use dedicated private stages, profiles and prefixes, preserving originals and
saves. Inspect Wine's Documents links: the game may read
`Documents/SimCity 4/SimCity 4.cfg` before applying `-UserDir`.

Use complete GOG game/language/support files and the 32-bit registry value
`HKLM\Software\Maxis\SimCity 4\Install Dir` pointing to the staged root.
The GOG installation script declares it in `registryKey9`; PortCellar does
not currently populate it. `Apps/SimCity 4.ini` must resolve the game data
relative to the staged Apps directory.

```text
-l:English -intro:off -CustomResolution:enabled -r1600x900x32 -w
-d:opengl -CPUCount:1 -CPUPriority:high -UserDir:<private Windows directory>\
```

Keep the SCGL artifact and LAA declarations. Use a distinct 1.1.641 app ID;
the existing catalog profile targets 1.1.610.
Restore the original staged DLL after a pointer-swap experiment.
Game executables, original DLLs, prefixes and raw private logs are not distributed.
