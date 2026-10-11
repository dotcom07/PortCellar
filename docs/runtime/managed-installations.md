# Managed standalone installations

`game installation` is implemented for macOS and standalone profiles, using the
existing core/CLI crates and staging behavior. Rust 1.89 or newer is required
for standard-library file locking. The first route creates new prefixes; it
does not enroll existing prefixes, share Steam prefixes, run detached, refresh
stages/reconcile saves, or download/verify whole runtime packages.
Supported backends are `auto`, `wine-d3-d`, `scgl`, and `dxmt`. Additional
runtime dependencies, DXVK, VKD3D and D3DMetal are refused until their inputs
can be explicitly bound. The legacy routes keep their existing backend support.

## Create and plan

```sh
mkdir -p .portcellar/installations
cargo run -- game installation create \
  --root .portcellar/installations/arknights-kr --id arknights-kr \
  --module arknights --variant windows-korea --profile wine-dxmt \
  --source-root /path/to/installed/game --wine /path/to/Wine/bin/wine64 \
  --dxmt-root /path/to/DXMT
cargo run -- game installation plan --root .portcellar/installations/arknights-kr
```

The parent must exist; creation refuses any existing installation directory
and any parent already inside a managed installation.
Wine must have a sibling `wineserver`. The source is a real local directory;
source and runtime remain outside the new installation. Creation captures an
exact catalog tuple and profile, SHA-256 of Wine and wineserver, and SHA-256 of
every selected DXMT DLL. It does not hash Wine's entire library/dependency closure
or verify signing, translation requirements or packaging. No third-party setup
script executes. See the [Arknights module](../../modules/arknights/README.md)
for source provenance and candidate limitations.

Plans inspect without creating prefix, stage, locks, logs or recovery files.
They are advisory; execution rechecks inputs after taking the OS lock.

## Owned state and execution

| Path under the installation | Purpose |
| --- | --- |
| `.portcellar-installation.toml` | Private binding and captured profile |
| `.portcellar-locks/installation.lock` | Stable OS lock outside prefix/stage |
| `prefix/` | Wine state and installer output |
| `stage/game/` | Game copy, applied files and durable game-local saves |
| `home/` | Controlled home, temporary, XDG and cache paths |
| `logs/` | Wine output and module logs |
| `recovery/` | Original/replacement files and durable change records |
| `.portcellar-session.toml` | In-flight/interrupted Wine session marker |

Keep the entire directory private and ignored. Prefixes can contain accounts
and game payloads. Core-owned prefix/home/temp values override profile settings;
children use a controlled environment. `G:` maps to the game copy and `L:` to
owned logs. The standard `Z:` host-root mapping is removed after initialization.
These controls reduce accidental access; Wine is not a host security sandbox.

```sh
cargo run -- game installation prepare --root .portcellar/installations/arknights-kr
cargo run -- game installation launch --root .portcellar/installations/arknights-kr \
  --timeout-seconds 3600
```

Prepare copies the game and refuses refresh/replacement of a populated stage
whose source fingerprint changed. Originals and saves remain intact. It does
not bootstrap the prefix or start the game; if a prefix exists, it can probe
server quiescence. Launch prepares the prefix, applies typed module registry
settings, installs architecture-selected DXMT files beside the selected
executable when requested, and runs
foreground. DXMT and game-specific DLL exceptions are scoped to the profile.

The one lock covers this installation's fixed child resources. Filesystem aliases
refer to the same lock file; separate installations can run independently.
Locks are never unlinked or stolen. A launcher exit alone does not end ownership:
the selected `wineserver -w` must finish. CLI timeouts are per subprocess,
between 1 and 86400 seconds. Core returns numeric exit codes; the CLI reports
nonzero codes as errors. Signals/timeouts
are failures. Process/session completion does not verify rendering or gameplay.

## Installer experiment

```sh
cargo run -- game installation plan --root .portcellar/installations/arknights-kr \
  --installer /path/to/setup.exe --installer-arg /quiet
cargo run -- game installation install --root .portcellar/installations/arknights-kr \
  --installer /path/to/setup.exe --installer-arg /quiet --timeout-seconds 3600
```

Use the module's `installer-baseline` profile without a DXMT payload for a
separate installer binding. An empty input directory satisfies its required
source input. The selected EXE is hashed, copied inside the owned prefix,
rechecked and invoked by its `C:` path; the original is preserved. Install does
not launch a game or rebind the source automatically. Inspect the output, then
explicitly create a game binding from the installed directory. Keep the original
installer experiment. Do not direct an installer at preserved input files.
The installer's own host-path/network behavior still requires review.

## Exact file changes

```sh
cargo run -- game installation replace --root .portcellar/installations/arknights-kr \
  --target helper.dll --artifact /path/to/reviewed/helper.dll \
  --original-sha256 ORIGINAL_SHA256 --replacement-sha256 REPLACEMENT_SHA256
cargo run -- game installation restore --root .portcellar/installations/arknights-kr \
  --target helper.dll
```

Supply actual 64-character lowercase hashes. `--original-sha256 absent` adds
a file only when its target is absent. Targets are relative to the staged game;
traversal, absolute/drive paths, symlinks, reserved paths and unknown preimages
are refused. Original and replacement bytes are retained and checked. The change
record is flushed before same-directory activation. Repeating an unchanged
operation is idempotent. Restore accepts only the applied or already restored
content; updater/user changes are preserved and refused. Added files are removed
only when their exact owned postimage remains, retaining the recovery copy.

Interrupted copies can leave partial retained files or pending activation.
Validated retained copies can complete activation; mismatching partial files
remain for manual inspection. Do not erase recovery files to force a retry.
Helper-specific sibling backups and dependency order belong to the module.

## Recovery and legacy routes

An interrupted supervisor or unconfirmed server shutdown leaves a marker that
blocks subsequent mutations. Close the owned game normally, then run:

```sh
cargo run -- game installation recover --root .portcellar/installations/arknights-kr \
  --timeout-seconds 30
```

Recovery acquires ownership, rechecks inputs and waits for the selected Wine
server before clearing the marker. It does not kill unknown processes, reset a
prefix or restore saves. A changed engine requires manual reconciliation.
External Wine tools do not honor PortCellar locks; do not launch them against
this prefix concurrently. Legacy runners and filesystem mutation entry points
refuse managed resources. Existing unmanaged SC4/Isaac/Steam routes retain
their limitations; the broader migration contract remains in
[foundation decisions](../architecture/foundation-decisions.md).

Synthetic core/CLI tests use temporary directories and shell fixtures, not
downloaded Wine, games or accounts. They exercise source/save preservation,
planning purity, exact changes, stale-restore refusal, Unicode paths/settings,
typed registry arguments, runtime/payload changes, architecture selection,
two-process contention and supervisor interruption. They do not prove a real
Wine build's server semantics or game compatibility.
