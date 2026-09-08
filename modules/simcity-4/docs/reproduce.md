# SimCity 4 developer reproduction guide

This guide reconstructs and checks the public SCGL source change for the
recorded SimCity 4 Deluxe 1.1.610.0 investigation. It does not install the
game, Wine, CrossOver, or a PortCellar module.

The source and patch checks below are runnable from a fresh clone. The game-run
steps bind your own installation and Wine engine to a private profile. This
repository does not distribute either dependency or a built DLL.

**Bounded game observation:** on 2026-09-08, a fresh public-source build reached
the region menu and Getting Started Tutorial with the WineD3D Vulkan setting
in section 5. The default WineD3D OpenGL path failed during initialization.
Screenshots were captured for user review; this is not full gameplay validation.

## Core finding

1. **Symptom:** SimCity 4 1.1.610.0 showed white textures in the Getting Started
   Tutorial and could terminate while rendering.
2. **Root cause:** SCGL's C++ virtual-function-table order did not match the
   MSVC-built game's binary ABI. The MinGW build retained a different order for
   overloaded methods, so calls could dispatch to the wrong overload.
3. **Affected slots:** `0x6c`, `0x70`, `0xd8`, `0xdc`, `0xe0`, and `0xe4`.

The six slots cover the affected `TexEnv` and `TexStageCombine` overloads. The
patch is an aggregate compatibility change, so the slots are checked together
with its other initialization and renderer changes below. This finding is
separate from the WineD3D Vulkan workaround described in section 5.

## 1. Inspect the module

Run from the repository root:

```sh
cargo run -- game module --root modules --id simcity-4
```

The command validates the descriptor and its referenced profile. It does not
install or launch the game.

## 2. Reconstruct the pinned source change

Initialize only the SCGL submodule, then verify the expected upstream commit:

```sh
git submodule update --init upstreams/scgl
test "$(git -C upstreams/scgl rev-parse HEAD)" = \
  dc80faec59980da7436e792171e3ce55778f41cd
```

Check and apply the parent-repository patch in the disposable submodule
checkout:

```sh
git -C upstreams/scgl apply --check \
  "$PWD/modules/simcity-4/patches/scgl/simcity4-1.1.610-abi.patch"
git -C upstreams/scgl apply \
  "$PWD/modules/simcity-4/patches/scgl/simcity4-1.1.610-abi.patch"
```

The patch is an aggregate compatibility change. The six texture-vtable
entries are only one part of it; initialization, mode handling, context
selection, buffer regions, and diagnostics are also changed. Keep this
checkout disposable so the upstream submodule remains clean for another
attempt.

## 3. Build SCGL

The upstream CMake project requires CMake 3.22 or newer, Ninja, and an i686
MinGW C and C++ compiler. On macOS or another Unix host, configure and build
the disposable patched checkout like this:

```sh
cmake -S upstreams/scgl -B .portcellar/build/scgl -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_C_COMPILER=i686-w64-mingw32-gcc \
  -DCMAKE_CXX_COMPILER=i686-w64-mingw32-g++ \
  -DSCGL_DIAGNOSTICS=OFF \
  -DSCGL_SIMCITY4_610_ABI=ON \
  -DSCGL_STATIC_RUNTIME=ON
cmake --build .portcellar/build/scgl
file .portcellar/build/scgl/SCGL.dll
```

The expected file type is a 32-bit Intel Windows DLL (`PE32`). Record the
compiler, CMake, Ninja, patch, and resulting DLL versions or hashes with any
published run. Do not infer them from the historical corrected DLL hash.

The current source build was freshly checked on 2026-09-08 with CMake 4.1.1,
Ninja, and GCC 16.1.0. It produced a PE32 DLL in a disposable checkout. This
is build evidence only; it does not establish game compatibility.

## 4. Check a built DLL

Once a locally built DLL exists, run both checker modes:

```sh
python3 modules/simcity-4/tools/check-scgl-texture-abi.py \
  .portcellar/build/scgl/SCGL.dll
python3 -O modules/simcity-4/tools/check-scgl-texture-abi.py \
  .portcellar/build/scgl/SCGL.dll
```

The checker verifies the six declared vtable slots. It does not prove that the
DLL builds from this source, that every SCGL ABI entry is compatible, or that
the game renders correctly.

On 2026-09-08, the unpatched and patched builds were both checked in separate
disposable checkouts. The unpatched DLL failed at slot `0x6c` under normal and
optimized Python; the patched DLL passed both modes. These local hashes identify
the toolchain outputs and are not release binaries:

```text
unpatched: 14b764a92ae0955e62e91eef6ca435150d1bb98901c9042cff9b7be38a09f461
patched:   9adbea7f0a4974b6a41fec34237a8fa1aa8ae18b38e6eeaf7f1feebaff7ab35c
```

## 5. Bind your installation and launch with PortCellar

Run these steps from the same disposable PortCellar clone used for the build.
Requirements: Rust/Cargo, Python 3.11+, and an installed Wine Staging 11.10 engine
with working macOS/Rosetta support. The repository does not install Wine. Set
`PORTCELLAR_WINE` to the engine's `wine` executable (not its `.app` directory);
keep its sibling Wine components together. The tested bundle includes Vulkan
support through MoltenVK; the workaround below requires that support. Other
Wine versions are untested.

Set the two machine-specific paths. `SC4_GAME_DIR` must contain both `Apps/`
and the game data files, and must be the matching Windows 1.1.610.0 install:

```sh
export SC4_GAME_DIR='/absolute/path/to/SimCity 4 Deluxe Edition'
export PORTCELLAR_WINE='/absolute/path/to/wine/bin/wine'
export PORTCELLAR_STATE_ROOT="$PWD/.portcellar"
export PORTCELLAR_WINEPREFIX="$PORTCELLAR_STATE_ROOT/prefixes/sc4-community"
mkdir -p "$PORTCELLAR_WINEPREFIX"
```

Use a new prefix path for this experiment. Wine can create links to host user
folders; the `-UserDir` below explicitly puts SC4 user data in the private state
directory so existing cities and settings are not reused. The runtime stages a
copy of the installation, adds the new DLL, and applies LAA only to that copy.
Do not run two sessions against the same prefix or state directory.

Create a private launch profile from the public module profile (Python 3.11+):

```sh
python3 - <<'PY'
import json
import os
from pathlib import Path
import tomllib

root = Path.cwd().resolve()
state = Path(os.environ['PORTCELLAR_STATE_ROOT']).resolve()
game = Path(os.environ['SC4_GAME_DIR']).resolve(strict=True)
assert (game / 'Apps/SimCity 4.exe').is_file(), 'Check SC4_GAME_DIR'
dll = root / '.portcellar/build/scgl/SCGL.dll'
assert dll.is_file(), 'Build SCGL first'
userdir = state / 'sc4-user'
userdir.mkdir(parents=True, exist_ok=True)
windows = lambda p: 'Z:' + str(p).replace('/', '\\')
quote = lambda s: json.dumps(s, ensure_ascii=False)
base = (root / 'modules/simcity-4/profiles/windows-1.1.610.toml').read_text()
base = base.replace('windows_exe = "SimCity 4.exe"',
                    'windows_exe = "Apps/SimCity 4.exe"')
args = ['-l:Korean', '-intro:off', '-CustomResolution:enabled',
        '-r1600x900x32', '-w', '-d:opengl', '-CPUCount:1',
        '-CPUPriority:high', '-UserDir:' + windows(userdir) + '\\']
base += '\nwindows_install_path = ' + quote(windows(game)) + '\n'
base += 'launch_arguments = ' + json.dumps(args, ensure_ascii=False) + '\n'
base += '\n[[runtime_artifacts]]\nid = "scgl-simcity4-1.1.610"\n'
base += 'source_path = ' + quote(str(dll)) + '\n'
base += 'target_path = "Plugins/SCGL.dll"\narchitecture = "pe32-i386"\n'
base += '\n[[binary_patches]]\ntarget_path = "Apps/SimCity 4.exe"\n'
base += 'kind = "large-address-aware"\n'
tomllib.loads(base)
profile = state / 'profiles/sc4.toml'
profile.parent.mkdir(parents=True, exist_ok=True)
with profile.open('x') as output:
    output.write(base)
print(profile)
PY
cargo build --locked
cargo run --locked --quiet -- game launch \
  --from-profile "$PORTCELLAR_STATE_ROOT/profiles/sc4.toml" \
  --mode wine-direct --dry-run
```

The profile uses Korean for the recorded installation; choose a language present
in your installation if necessary. Before staging, the dry-run can display the
original executable path. It does not create the copy. The actual execution
command must display the executable under the chosen state directory
(`.portcellar/runtime/` with the default above).

Use WineD3D's Vulkan backend for the game's separate DirectDraw capability
initialization. SCGL still handles game rendering through OpenGL (`-d:opengl`).
This process-local workaround passed the isolated probe and game scenarios
below; it does not repair Wine's OpenGL implementation:

```sh
WINE_D3D_CONFIG=renderer=vulkan \
PORTCELLAR_WINEDEBUG='+timestamp,+pid,+loaddll,+seh' \
cargo run --locked --quiet -- game launch \
  --from-profile "$PORTCELLAR_STATE_ROOT/profiles/sc4.toml" \
  --mode wine-direct --wait --capture-wine-log \
  --launch-timeout-seconds 60
```

`--wait` checks process survival for ten seconds, not rendering. Logs are
appended under the prefix's `portcellar/logs/`; use a fresh prefix so older
exceptions cannot be mistaken for this run. Private smoke records are under
`.portcellar/smoke/`. A failed stage is retained; use a new state directory after
reconciling saves instead of deleting or forcing replacement of an existing copy.

Open the region menu and Getting Started Tutorial. Capture both the terrain and
the interface, and report the exact scenario and observer. Record the game,
SCGL and Wine hashes, source revisions, host/macOS versions, arguments and
outcome. Successful launch alone does not establish that the reported texture
problem is fixed. Long sessions, save/load, mods, large cities and Steam
compatibility remain separate checks.

## Fresh game-run observation (2026-09-08)

The public checkout at `183144c2773e1ee47f80d561da524d54789578a6`, with the
pinned SCGL source and aggregate patch above, produced DLL SHA-256
`0d0e4bcb52a6a4acda38c80af80e3c1ed6b893ffd3b0d94cfbdbcf29cf90ac2e`.
Both texture ABI checker modes passed. This is a separate build from the hashes
in section 4.

On an Apple M4 Pro running macOS 26.6.2, Wine Staging 11.10 loaded this DLL
from the PortCellar-staged game in a new prefix. The ten-second process check
passed, but only a black initialization window was captured; no region or
tutorial rendering was observed. A debugger inspection found an access
violation with recorded exception address `0x7bf31239` and a secondary fault
in Wine's exception setup. Additional WineD3D tracing ended during framebuffer
capability checks. These observations do not establish the originating cause.

A process-local `WINE_D3D_CONFIG=renderer=no3d` experiment instead displayed
"Could not initialize Direct Draw." It is not a working workaround.
Historical rendering observations remain separate from this failed fresh run.

The same public-source game, DLL, prefix and arguments then reached the region
menu and Getting Started Tutorial with `WINE_D3D_CONFIG=renderer=vulkan`.
The agent captured terrain and interface screenshots from that process, and
the user confirmed the displayed run worked. This is a bounded manual
observation, not a claim that every graphical issue is fixed. The loaded-DLL
log identifies the staged fresh SCGL build and confirms WineD3D selected Vulkan. No Wine engine
binaries were modified.

### Isolate the initialization failure without game files

[probe-directdraw.c](../tools/probe-directdraw.c) calls only `DirectDrawCreate`
and `GetCaps`. It does not load SCGL or need SC4. Build both architectures:

```sh
mkdir -p .portcellar/build
i686-w64-mingw32-gcc -O0 -Wall -Wextra \
  modules/simcity-4/tools/probe-directdraw.c -lddraw \
  -o .portcellar/build/probe-directdraw32.exe
x86_64-w64-mingw32-gcc -O0 -Wall -Wextra \
  modules/simcity-4/tools/probe-directdraw.c -lddraw \
  -o .portcellar/build/probe-directdraw64.exe
```

After stopping the game, run one probe at a time in the test prefix. Substitute
`64` for `32` to compare architectures, or `renderer=vulkan` for `renderer=gl`
to compare WineD3D backends:

```sh
WINEPREFIX="$PORTCELLAR_WINEPREFIX" WINE_D3D_CONFIG=renderer=gl \
WINEDEBUG='+timestamp,+pid,+seh' \
  "$PORTCELLAR_WINE" "$PWD/.portcellar/build/probe-directdraw32.exe"
```

Observed on the same host, engine and test prefix:

| Probe | Result |
| --- | --- |
| 32-bit, default OpenGL backend | Access violation inside `DirectDrawCreate`; did not return to the probe |
| 64-bit, default OpenGL backend | Both calls returned `00000000` |
| 32-bit, Vulkan backend | Both calls returned `00000000` |

This isolates an independently reproducible failure in the installed Wine
engine's 32-bit DirectDraw/OpenGL initialization path. SC4, SCGL and the
PortCellar launcher are not required to trigger it. The precise instruction
that first corrupts the stack has not been identified, and this comparison
does not explain every historical successful run. Vulkan avoids this blocking
path in the tested scenarios; it does not establish general Wine compatibility.
