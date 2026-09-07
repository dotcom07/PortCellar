# SimCity 4 first public release plan

Date: 2026-09-07. Status: preparation only; nothing has been published by this
review. This plan complements the [architecture proposal](../architecture/portcellar-plan.md).

## Release purpose

Publish a bounded technical finding: the MinGW-built SCGL texture vtable did not
match the ABI used by the tested Windows SimCity 4 executable. Publish the source
change, a reliable binary check, reproduction instructions, and evidence with
clear limits.

The community entry point should be `modules/simcity-4/`, followed by an immutable
tag or GitHub Release for exact reproduction. The repository root explains
PortCellar to people who choose to explore further. Community posts should
describe the SC4 problem and finding without an LLM/platform sales pitch.

## Evidence reviewed so far

The existing investigation documents Windows SimCity 4 **1.1.610.0, PE32/i386**,
Wine Staging **11.10**, and Apple Silicon/macOS. Saved diagnostic metadata
identifies an M4 Pro and macOS 26.6.2 (25G83), with translated x86-64 Wine.
These identify a historical environment; they are not yet a complete,
reconstructible runtime build manifest.

The imported SCGL base refers to the pinned upstream revision
`dc80faec59980da7436e792171e3ce55778f41cd` from
[nsgomez/scgl](https://github.com/nsgomez/scgl). Its modifications are stored in
`modules/simcity-4/patches/scgl/simcity4-1.1.610-abi.patch`. The patch, checker,
and investigation documents are imported into PortCellar. This does not imply
that a buildable source checkout was initialized or freshly verified here.

| Finding | Evidence class | Publication limit |
| --- | --- | --- |
| The game passes scalar mode `1` through byte offset `0x70` | Saved game disassembly and decoded driver table | Specific executable/build |
| The old SCGL table places the pointer overload there | Old DLL symbol/table inspection | Specific MinGW DLL |
| The saved fault dereferences address `0x1` in `glTexEnvfv_Exec` | Saved LLDB excerpt and SCGL forwarding code | Supports the affected dispatch failure |
| Historical old DLL failed; historical corrected DLL passed the six-slot check in ordinary Python | [Dated checker verification below](#checker-verification-record-2026-09-07) | Historical pre-import checker result; no fresh old/new DLL regression has been run with the imported checker |
| Tutorial terrain was restored and the tutorial ran | Explicit user confirmation recorded in the investigation | Historical human observation, not automated visual verification |
| Previous access violation was not reproduced after the correction | Historical investigation report | Not proof that no future crash is possible |
| A diagnostics-enabled run failed during initialization | Historical investigation report | Exclude it from successful runtime results |

The separate SCGL binary audit within this planning review recorded these local
artifact SHA-256 values. The coordinating review independently recalculated both
DLL digests and repeated their checks. The original EXE digest was recorded by
the SCGL audit. These identify examined inputs; they are not release assets or
download promises.

```text
Original game EXE:
8810169790a3ebcbe656205f2d339dfa703c87c4964a51b8417d4a029ebc99bc

Old SCGL DLL:
9d1ab0fce972fd7ef3a5c15b522dd271ba91ccf667501136346e33f339784954

Corrected SCGL DLL:
f197a05fc36479383b0d1bae3d36e6fe662555d65cf08ed92bf2004ac87a6152
```

The original EXE is not LAA-enabled; the historical runtime profile requests LAA
on a staged copy. Record both original and executed hashes for release evidence.
Do not upload either game executable. The current local installation layout has
also changed since the historical profile; fingerprint and rerun the selected
release installation instead of inheriting its old result.

### The six affected entries

These are byte offsets within the tested 32-bit vtable, not portable slot
numbers for every compiler or version.

| Byte offset | Old DLL dispatch | Required/corrected dispatch |
| --- | --- | --- |
| `0x6c` | `TexEnv` scalar `int` | `TexEnv` color `const float*` |
| `0x70` | `TexEnv` color `const float*` | `TexEnv` scalar `int` |
| `0xd8` | Combine mode | Combine scale |
| `0xdc` | Combine source | Combine operand |
| `0xe0` | Combine operand | Combine source |
| `0xe4` | Combine scale | Combine mode |

At the affected call, passing mode `1` to the pointer overload can forward
address `0x1` to `glTexEnvfv`. This supports the narrow ABI explanation. Do not
turn it into a universal claim about every MinGW/MSVC overload group.

### Two release blockers discovered during review

**The checker defect is fixed in the imported script.** Critical validation uses
explicit errors, so Python optimization cannot remove the checks. The script
still needs a fresh old/new DLL regression under normal and optimized Python
before it becomes a release gate. Document the PE32/i386, retained-symbol,
MinGW symbol-name, and vtable-address-point assumptions. Unsupported/stripped
inputs must fail clearly.

**The aggregate patch is larger than the headline fix.** It also includes earlier
1.1.610 interface changes, legacy context/mode handling, buffer-region behavior,
and diagnostics. Some interface changes are unconditional even when the 610 ABI
option is disabled. Do not rename the entire patch to `texture-vtable-abi.patch`
and describe every line as a six-entry fix.

Prepare a reviewable ordered patch series, or retain the aggregate patch with a
separately documented six-entry delta against a specified baseline. Record which
prerequisite changes were already present in the failing DLL. Keep unrelated
Wine compatibility patches separately identified in the runtime manifest.

### Checker verification record: 2026-09-07

This read-only check was run against the legacy worktree by the SCGL audit and
repeated by the coordinating reviewer. It is separate from the publication and
privacy audit, and it did not launch a game or rebuild a DLL.

- Python: 3.13.1; GNU `i686-w64-mingw32-nm`: Binutils 2.46.1.
- Checker: legacy `modules/simcity-4/tools/check-scgl-texture-abi.py`, SHA-256
  `90e57733227d93b75e89223a38b119a450e009294363a8b3674a6e5226240c4e`.
- Inputs: old/corrected DLL SHA-256 values recorded above.
- `PYTHONOPTIMIZE` was removed from the child environment; `-B` disabled bytecode
  writes. Explicit `-O` was used only for the optimized cases.

Equivalent commands, run from the legacy repository with `SCGL_OLD_DLL` and
`SCGL_NEW_DLL` bound to the examined local artifacts:

```sh
env -u PYTHONOPTIMIZE python3 -B modules/simcity-4/tools/check-scgl-texture-abi.py "$SCGL_OLD_DLL"
env -u PYTHONOPTIMIZE python3 -B -O modules/simcity-4/tools/check-scgl-texture-abi.py "$SCGL_OLD_DLL"
env -u PYTHONOPTIMIZE python3 -B modules/simcity-4/tools/check-scgl-texture-abi.py "$SCGL_NEW_DLL"
env -u PYTHONOPTIMIZE python3 -B -O modules/simcity-4/tools/check-scgl-texture-abi.py "$SCGL_NEW_DLL"
```

| Input | Ordinary Python | `python -O` |
| --- | --- | --- |
| Old DLL | Exit 1: mismatch at `0x6c` | Exit 0: incorrect PASS |
| Corrected DLL | Exit 0: PASS | Exit 0: PASS |

The old ordinary run reported `AssertionError` for the color overload at `0x6c`.
The other three runs printed `PASS: all six SimCity 4 1.1.610 texture ABI slots match`.
These are dated observations of the pre-import checker defect, not current
verification. The imported script's normal and optimized no-argument checks pass
as argument-validation checks; rebuildable public old/new inputs remain a
milestone A gate.

## What the public module should contain

```text
modules/simcity-4/
  README.md
  module.toml
  profiles/
  patches/scgl/
  tools/check-scgl-texture-abi.py
  tests/
  evidence/
  docs/investigation.md
```

The module README must provide two clear paths:

- **Players:** exact requirements, download availability, installation/run steps,
  save protection, limitations, and reporting results.
- **Developers:** root cause, slot table, source baseline/patches, compiler and
  CMake settings, old/new binary checks, and scenario reproduction.

The current v1 module descriptor is not a copy of a private runtime profile. It
identifies module identity, variants, store metadata, and profile references.
Executable artifacts and scenarios belong to a future execution and evidence
contract. The user's game path, engine location, and prefix are resolved
privately.

The tested SCGL plugin is staged at `Plugins/SCGL.dll`. Do not substitute the
historical `SimGLRef.dll` name as the injection destination. Verify the selected
game's plugin-directory configuration in the reproduction procedure.

The release build should explicitly select an i686 MinGW compiler and pin or
record its version, CMake version, Release configuration, diagnostics OFF,
610 ABI ON, and static-runtime setting. Do not depend on a previous CMake cache
or the maintainer's toolchain path. Static MinGW linkage does not eliminate
Windows/Wine UCRT and OpenGL/system dependencies.

## Two publication milestones

### A. Reproducible technical source publication

This can proceed before a turnkey runtime download exists, when all of these
checks are complete:

- Exact upstream commit and complete ordered modifications are available publicly.
- A clean build reproduces the intended binary behavior with explicit toolchain
  selection; the check fails on the old build and passes on the corrected build
  under both normal and optimized Python.
- A public manifest identifies original/executed game hashes, runtime artifact,
  Wine patches, SCGL build, launch arguments, display settings, prefix setup,
  and required components. Missing required runtime/component acquisition
  references, source/patch inputs, or build configuration block this milestone.
  Disclose uncertainty only when it does not prevent reconstruction of the tested
  configuration. Users obtain the matching game legally; the project does not
  distribute its executable or data.
- The selected release configuration is rerun through launch, region view, and
  Getting Started Tutorial, with the observation method recorded. Preserve
  before/after evidence of the texture failure and the bounded result.
- Selected debugger/slot evidence is sanitized and reviewable without requiring
  access to private logs. Do not upload the full game disassembly.
- Source licensing, third-party notices, and changed-file notices are complete.
- Public wording distinguishes historical human observations, binary checks,
  and fresh release validation. Documentation links work at the release tag.

Use an explicit **source-only** label if players must still build or provide a
runtime. It is a technical finding with reproduction material, not a ready-to-run
player download. Wider gameplay remains unverified unless actually tested.

### B. Experimental player package

Add a player download after:

- The CLI/runtime/module combination installs and runs on a clean eligible Mac
  without Rust, MinGW, or a Wine build on the player's machine.
- Package dependencies, redistribution rights, corresponding source, integrity
  hashes, relocation, and signing/Gatekeeper behavior have been checked.
- Installation preserves original game files, existing plugins, and saves.
  Updates/uninstall retain user data, and a failed update has a recovery route.
- Another user can follow the instructions and report the exact scenario result.

Runtime and module assets remain logically and version-wise separate. Name
artifacts with their actual architecture and versions; do not call a mixed
arm64-CLI/x86_64-Wine bundle simply "arm64 runtime" without explaining its
Rosetta dependency. Keep source packages and notices alongside binary releases.

Recommended public title:

> SimCity 4 compatibility module — initial experimental release

The CLI does not need a broad `0.1.0` product launch merely to publish this SC4
finding. A module-specific tag can anchor it while the broader project develops.

## README wording after milestone A passes

Use this status, qualified by the evidence linked directly below it:

> Experimental — scenario verified

The first screen should identify **Windows SimCity 4 1.1.610.0 (x86)**, the tested
host/OS, exact runtime, and profile. It should say which distribution/edition was
actually identified. Do not label every Deluxe edition or the Steam release
tested from a version string alone.

Show the exact verified scenarios and observation methods. Alongside them show:

- Long gameplay sessions: not yet verified.
- Save/load across varied cities: not yet verified.
- Other rendering paths and display configurations: not yet verified.
- Plugins, DLL mods, and large developed cities: not yet verified.
- Complete SCGL ABI and complete game compatibility: not established.
- Steam SC4: not established by this standalone configuration.

Keep the vertex-format/fog ABI differences and diagnostics-build failure in
known limitations. Do not display `Supported`, `Fully working`, or `Compatible`
as an unqualified module-wide status.

## Community publication

GitHub is the canonical source for code, evidence, and releases. One technical
Simtropolis thread is the intended main discussion, then a shorter r/simcity4
post and a brief developer-feedback request in SC4Evermore Discord. Link to the
same module/release; do not maintain separate patch attachments on each service.

Suggested Simtropolis title after the reproduction gates pass:

> Fixed: SCGL white textures / tutorial crash under Wine on Apple Silicon

Suggested body structure:

1. The tested Windows build reached the region menu but tutorial entry produced
   white rendering and could crash.
2. The affected virtual call dispatched integer mode `1` to a pointer overload.
3. Show the six-offset table and link the bounded source change/check.
4. State the tutorial result and identify human versus automated verification.
5. State the unverified gameplay, save/load, plugin, and ABI scope.
6. End with the GitHub source/evidence link.

A concise closing statement is:

> This is not a claim of full SimCity 4 compatibility. The linked evidence covers
> the launch path, region view, and Getting Started Tutorial in the tested
> configuration. Long sessions, broader gameplay, save/load, other rendering
> paths, plugins, and the remaining SCGL ABI have not been comprehensively tested.

Do not post duplicate announcements to the modding and troubleshooting forums.
The proposed modding forum placement must be checked against current forum rules
before posting: this review received HTTP 403 from Simtropolis. The
[SC4Evermore site](https://www.sc4evermore.com/) exposes Downloads and a Discord
link, but internal Development/Helpdesk channel availability was not verified.
Reddit's current rules were not checked. No posts or messages were sent.

Consider SC4Evermore Downloads only after package layout, licensing, update
instructions, and external testing are stable. An upstream SCGL PR should contain
the bounded fix, regression proof, and prerequisites, with no project promotion.
Check upstream issue/PR status and contribution guidance when preparing that PR.

## Community reports and ongoing maintenance

Ask contributors for the module/release, game distribution/version, architecture,
runtime digest/version, macOS version, SCGL digest, selected profile, scenario,
duration, result, mods, and observation method. Ask for sanitized excerpts only;
never request a game upload, Steam account data, or a complete raw prefix.

A report is `community-reported` until reviewed. Preserve conflicting results
and their configurations. When asked about an untested route, say:

> I haven't tested that configuration yet.

Use new reports to select the next bounded scenarios: sustained city play,
save/reload, alternate display modes, representative cities, then explicit mod
combinations. None of those are prerequisites for honestly publishing the initial
technical finding; they are prerequisites for expanding its compatibility claims.

## Evidence provenance and limits of this review

The original review checked the legacy SCGL source, patch, binary checker, saved
LLDB/disassembly excerpts, build metadata, and investigation notes. This turn
reconciled the documentation and checked the current source and no-argument
checker behavior; it did not newly examine private LLDB or game data. The binary
checker was historically exercised against the available old and corrected DLLs
using the pre-import checker; no gameplay was rerun. Raw local diagnostics remain
private, and the review did not create a distributable runtime or complete a
license compliance audit.

SCGL declares [LGPL-2.1-or-later](https://github.com/nsgomez/scgl), including
requirements for modified source. Its README identifies gzcom-dll as Expat and
Scion as LGPL; collect their applicable notices and review static-runtime
obligations before distributing binaries. Do not replace those licenses with the
future PortCellar root license.
