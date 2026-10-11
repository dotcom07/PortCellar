# Arknights compatibility mechanisms

Reference revision:
[`aa3a8ed7d76d18c9df9ece55026f143f0b98ba1e`](https://github.com/LuMiSxh/Arknights-MacOS-Client/tree/aa3a8ed7d76d18c9df9ece55026f143f0b98ba1e).
The observations below come from source inspection. They have not been
reproduced in PortCellar. No upstream source or binary is included here.

## Vuplex browser

The target is
`Arknights_Data/Plugins/x86_64/VuplexWebViewChromium/Vuplex WebView.vuplex`.
The reference checks that the official helper contains its supported
`vx-accelerated-paint-disabled` argument before replacement. Its wrapper
preserves incoming arguments, appends missing browser flags, and launches the
official helper as `Vuplex WebView.original.helper.vuplex` beside it. A generic
backup stored only in PortCellar's recovery directory is insufficient: that
sibling file must also be placed for the wrapper to find it.

Source comments explain three separate mechanisms: select the CPU `OnPaint`
transfer to avoid concurrent shared-texture writes by Unity and CEF; disable
CEF's AsyncDns path that depends on an unsupported Wine socket ioctl; provide
process-local `userenv.dll` AppContainer compatibility. Extra WebGL/rasterization
flags are tied to the reference's browser/payment observations. These are
upstream explanations, not established causes for a new Korean build.

For an experiment, identify and record the exact official helper and dependent
files first. Build the pinned wrapper and `UserenvCompat.c`, retaining MPL-2.0
notices and build provenance. Place the official sibling helper with an
`absent` precondition and its original SHA-256, replace the entry point with
exact preimage/postimage hashes, and place `userenv.dll` only with a verified
precondition. Restore the entry point before removing its sibling dependency.
Do not apply the policy to unrelated CEF or Vuplex applications by name.

The wrapper expects `L:` for its browser log. The managed route maps that drive
to installation-owned logs. Its runtime-dependent high-resolution controls
need a separate experiment.

## Notices window

The reference replaces `PlatformProcess.exe`, preserves the official file as
`PlatformProcess.original.helper.exe`, and supplies
`PlatformProcessWindowBridge.dylib`. The native bridge adjusts macOS window
transparency/layering for the black notices surface reported by that project.
It is a game-specific native bridge, not a global Wine renderer correction.
Review the helper's child-process and library-loading behavior and retain its
required sibling paths before trying it. PortCellar does not automatically
inject native bridges through a profile environment variable.

## Common ownership versus policy

PortCellar owns installation paths, locks, subprocess environment, game copies,
SHA-256 preconditions, durable change records and conditional restoration.
The module owns helper selection, placement dependencies, arguments, eligibility
and scenario evidence. The same file-change operation can later serve an SCGL
DLL or another game without making every game use Arknights policy.

Before a publisher update, restore known applied entry points while current
hashes still match, preserving all recovery files. After an update, inspect
the new inputs and create a reviewed comparison. The first managed route
refuses stage refresh; it does not reconcile saves or run publisher updates
automatically.

## Licensing

The client and RuntimeSupport sources are MPL-2.0. Any future copied or modified
files need their original notices, attribution and applicable source obligations.
Wine, DXMT and other runtime components have separate licenses and provenance.
The first-party Rust mechanisms here do not relicense those components.
