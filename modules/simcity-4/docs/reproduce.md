# SimCity 4 developer reproduction guide

This guide reconstructs and checks the public SCGL source change for the
recorded SimCity 4 Deluxe 1.1.610.0 investigation. It does not install the
game, Wine, CrossOver, or a PortCellar module.

The source and patch checks below are runnable from a fresh clone. A fresh game
run is still a release prerequisite: this repository does not publish a built
DLL or the private game/runtime paths needed to reproduce that run.

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
cmake -S upstreams/scgl -B /tmp/portcellar-scgl-release -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_C_COMPILER=i686-w64-mingw32-gcc \
  -DCMAKE_CXX_COMPILER=i686-w64-mingw32-g++ \
  -DSCGL_DIAGNOSTICS=OFF \
  -DSCGL_SIMCITY4_610_ABI=ON \
  -DSCGL_STATIC_RUNTIME=ON
cmake --build /tmp/portcellar-scgl-release
file /tmp/portcellar-scgl-release/SCGL.dll
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
  /tmp/portcellar-scgl-release/SCGL.dll
python3 -O modules/simcity-4/tools/check-scgl-texture-abi.py \
  /tmp/portcellar-scgl-release/SCGL.dll
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

## 5. Game-run evidence still required

A developer who has a legally installed matching game and a documented Wine
environment can extend this guide after the build manifest is complete. The
run record must identify the executable hash, SCGL hash, Wine and host
versions, display mode, prefix, launch arguments, and scenario. Preserve the
game installation and saves; use a disposable or recoverable test setup.

The minimum fresh observation is: launch the game, open the region menu, enter
Getting Started Tutorial, and record whether tutorial terrain renders and
whether the earlier access violation recurs. This is manual evidence and must
be labeled as such. Long gameplay, save/load, plugins, alternate display modes,
large cities, and Steam compatibility remain separate checks.

## Current publication boundary

The repository now contains a reproducible source build and checker path for
the patched DLL. It is not yet ready to claim a reproducible game run from the
repository alone. Remove this boundary after a fresh attributed game run has
been recorded.
