# Foundation decisions

Date: 2026-09-07. Status: selected implementation direction, not implemented
runtime guarantees. Reviewed against foundation commit `45da0cd`.
This document refines the [architecture proposal](portcellar-plan.md) where
specified below. The two existing crates remain the implementation boundary.

## Priority and actual gaps

Safe execution precedes expanding the public module execution API.

| Observed implementation | Required next behavior |
| --- | --- |
| `command.rs::run_plan` maps a signal to exit code zero | Signal termination returns an error, preserving normal numeric exit codes |
| `runtime/launch.rs::game_launch_plan` enables materialization | Every planning entry point is read-only; execution explicitly prepares state |
| `runtime/stage.rs` deletes an existing game copy before rebuilding | Refuse replacement initially; preserve the entire existing copy |
| Stage storage is keyed by `profile.app_id()` | Managed stages belong to an installation, allowing two copies of the same game |
| Mutation, installer, CEF patch, reset and launch paths lack a shared lock | A core-owned execution boundary serializes operations on the same resources |
| CEF wrapper detection uses a marker and shared temporary build paths | Inspect original and wrapper identities separately; serialize or isolate builds |

An installer dry-run test does not establish that all planning APIs are pure.
Existing tests and historical gameplay observations do not prove concurrency
safety, save preservation during refresh, or current Steam compatibility.

## Module descriptor v1: descriptive catalog first

Use `crates/portcellar-core/src/games/module.rs` for a small descriptor loader,
reusing `GenericGameProfile` and its parser. Do not add a crate, plugin host,
package resolver, or second implementation of profile settings. The existing
profile catalog remains available with its existing behavior.

The first new operations are read-only `game modules --root modules` and
`game module --root modules --id simcity-4`. They enumerate/validate descriptors
and display their referenced profiles. They do not launch, install, fetch,
select a user's installation, or execute module scripts.

Proposed complete example for the first descriptor:

```toml
format_version = "portcellar-module-v1"
id = "simcity-4"
name = "SimCity 4"
revision = 1

[[variants]]
id = "windows-1.1.610"
name = "Windows Deluxe 1.1.610.0 (x86)"

[[variants.profiles]]
id = "scgl"
path = "profiles/windows-1.1.610.toml"
```

The contract is deliberately limited:

- `format_version`, `id`, `name`, positive integer `revision`, and nonempty
  `variants` are required. Each variant requires `id`, `name`, and a nonempty
  list of profiles. Each profile reference requires `id` and `path`.
- A variant may additionally have `stores`, a map of store namespace to a
  nonempty string ID. Steam uses key `steam` and ASCII numeric IDs. Other store
  IDs remain opaque. Store metadata never grants ownership or enables Steam.
- Module, variant, profile-reference and store-namespace IDs match
  `[a-z0-9]+(-[a-z0-9]+)*`. Module ID must match its directory. Module IDs are
  unique per catalog, variant IDs per module, and profile IDs per variant.
  Names are nonempty Unicode text; host paths and game data support Unicode.
- All descriptor structures reject unknown fields, duplicate TOML keys, and
  unknown format versions. No shell hooks, URLs to execute, dependency ranges,
  compatibility badges, installation paths, or default execution route exist.
- Profile paths use `/`, end in `.toml`, and are relative to the module root.
  Reject absolute/drive/UNC paths, backslashes, empty components, `.` and `..`,
  NUL, and symlink components including the module directory. Require a regular
  file within the canonical module root. Loading never creates directories.
- Load referenced profiles through the existing parser, but reject unknown
  fields in module-referenced profile documents and nested typed records.
  Legacy direct-profile input keeps its existing compatibility behavior.
  Dynamic environment/option maps are not unknown-field errors; existing
  runtime validation still applies. Catalog loading is not execution approval.
- For v1 public modules, reject host-specific `windows_install_path`,
  `wine_engine_path`, and nonempty `runtime_artifacts`. Those artifacts currently
  contain local source paths; a digest-based artifact binding is a separate
  execution contract. Do not pretend the SC4 profile installs its patch.
- A profile requiring Steam must have a variant `stores.steam` equal to its
  legacy numeric `app_id`. A standalone profile needs no Steam ID. Optional
  Steam metadata on a standalone variant does not change execution policy.
- Resolve an exact `(module ID, variant ID, profile ID)` tuple. The descriptor
  does not overwrite `profile.app_id`, `bottle_name`, graphics settings, or
  executable names. Preserve the current direct-profile CLI during migration.

Variant identity describes a game build/distribution; profile identity selects
an execution route for that variant. Descriptor revision is a content revision,
not a format version or proof of integrity. Evidence must eventually retain
SHA-256 of the actual descriptor and profile bytes as well as these IDs.
An existing profile's `app_id` remains a legacy key until execution migration;
never use it as the new module, installation, or lock identity.

Do not ship `module.schema.json` before the loader consumes this model. When a
schema becomes useful to editors, derive it from these Rust types and add the
same accepted/rejected fixtures; JSON Schema covers parsed TOML, not TOML text.

## Prefix ownership and locks

Choose a macOS-first implementation using standard-library file locking
(`File::try_lock`, Rust 1.89 or newer, with an explicit workspace MSRV). Avoid
an additional locking dependency or a hand-written PID-file protocol. Other
hosts may inspect modules; managed execution there remains unsupported until
its filesystem and process behavior is tested.

A private installation binding owns one absolute prefix path and one stage
path. Different installation IDs referencing the same physical prefix share
the same lock. A shared Steam prefix is explicit and serializes all its games
in v1. Separate prefixes can execute concurrently.

The first private binding is TOML with required `format_version =
"portcellar-installation-v1"`, a user-selected slug `id`, `module_id`,
`variant_id`, `profile_id`, and absolute `prefix`, `source_root`, `engine_root`.
Store it at `.portcellar/installations/<id>.toml` (or the selected installed
state root). Reject unknown fields and duplicate IDs; validate the exact module
tuple. Derive its stage as `<state-root>/workspaces/<id>/game`. Runtime paths
are private inputs, not module fields or automatically exported evidence.
Validate source/prefix/stage separation and explicit ownership enrollment;
a path in a binding alone does not authorize modifying an arbitrary directory.
Managed execution resolves a temporary profile from module plus binding without
rewriting either file. Do not infer bindings from a game's Steam ID or silently
migrate an existing prefix. Legacy paths remain available for inspection and
explicit migration, but cannot bypass managed mutation guards.

Lock files live in a stable `.portcellar-locks/` sibling directory outside each
replaceable resource. Key them by the canonical resource path, using SHA-256
over its OS path bytes. Resolve an absent resource through its existing real
parent; reject symlink final targets. Existing aliases must resolve to the same
canonical key. Require a PortCellar-owned parent before creating locks or state.
Changing `PORTCELLAR_STATE_ROOT` cannot change the lock for the same prefix.
Test case-insensitive path aliases on macOS before claiming this invariant.

Acquire locks for both prefix and stage in sorted canonical-key order. Lock
failure returns busy before any mutation; unsupported locking returns an error.
Keep the locked file open throughout the operation. Never unlink a lock file,
steal it because of a timestamp, or use PID existence as the mutex. Kernel lock
release after owner death does not prove the Wine children have stopped.

Core execution owns the guards; putting guards only in CLI handlers is
insufficient. Recheck inputs, resource ownership, and live processes after
acquisition. Route launch/preparation, installer, dependencies, snapshot,
rollback, CEF patch/reset and their legacy aliases through this boundary.
Read-only plans acquire no locks and write nothing; they are advisory until
execution revalidates their inputs. Low-level `run_plan` is a process utility,
not proof that a prefix operation has been authorized or locked.

The first managed launch is foreground/supervised. Hold ownership until the
prefix's relevant Wine session is quiescent, not merely until the launcher exits.
If the supervisor dies, surviving owned processes block subsequent mutation.
Do not claim detached launches are protected by a guard dropped after spawn.
Keep detached managed launch unavailable until a tested supervisor retains
ownership. Do not silently fall back to an unguarded legacy command.

Advisory locks coordinate PortCellar processes, not external Wine or Steam
launchers. Prefix-scoped live-process checks are required; uncertain ownership
blocks mutation. Never kill an entire process group based on one matching PID
or a path substring. Stopping and cancellation must target verified owned
processes without waiting behind the session's exclusive lock.

## Replacement and save preservation

The immediate fix is to reuse an unchanged ready stage, create an absent stage,
and reject a refresh or invalid existing stage with a recovery message.
`PORTCELLAR_STAGE_REFRESH` must not bypass preservation. Build an absent stage
in a unique sibling directory, validate it, then activate it under ownership.
Failed staging leaves originals untouched and identifies its partial output.

Later replacement uses `prepare -> validate -> retain-current -> activate`:

1. Copy into a unique sibling on the same filesystem. Validate artifacts,
   executable, patch preimages and the completed marker before activation.
2. Write and flush a private operation journal with all paths and identities.
   Reject unexpected existing paths; never use recursive deletion to recover.
3. Retain the entire current stage/prefix under a unique backup name. Activate
   the replacement by rename. The two renames are not one atomic transaction.
4. On activation failure, restore the retained current directory only if its
   target is absent. On restart, reconcile journal and directories; an ambiguous
   state blocks launch and reports recovery paths. Retain backups after success.

Copying preserves the old files but does not make the active game's saves
current. Unknown save locations or modifications block automatic replacement;
do not activate a pristine copy and claim saves were preserved merely because
a backup exists. Until a reviewed module identifies save/config locations,
refresh requires explicit manual reconciliation. Prefix snapshots also cannot
cover host Documents reached through Wine symlinks, Steam Cloud, or external
Steam libraries; report coverage and exclusions. Never follow a prefix symlink
and copy or overwrite arbitrary host data as part of automatic recovery.

## Steam updates: observe, invalidate, reverify

Extend `analyze/steam_client.rs` and existing runtime Steam code. A future
`components/steam/` contains shared policy and scenarios only when those files
exist. No updater service or fork of the proprietary Steam client is needed.

Record client build/channel when observable, SHA-256 of Steam and original CEF
WebHelper executables, wrapper digest/policy revision, runtime digest, and
profile digest. Missing values are unknown. Keep paths and account state private.
Treat the shipped WebHelper and a PortCellar wrapper as distinct artifacts;
an overwritten target must not be paired with a backup from another version.

Compare fingerprints before launch, after client exit, and before any patch.
Changes set current eligibility to `needs-reverification`; old evidence remains
an immutable historical observation. A changed or missing fingerprint cannot
reuse a current pass. If files change during observation, mark the run unstable
and retry inspection after shutdown. Do not edit CEF while Steam updates it.

Permit an explicitly selected experimental run against changed inputs with the
uncertainty recorded. Recheck client start, WebUI, interactive login, library,
an already-owned installed game's launch, and prefix-owned shutdown separately.
Login-required is user action, not rendering failure. Client health alone does
not verify a game. Tests use synthetic logs/identities; no CI account is needed.

Recovery initially means restoring an exact PortCellar wrapper/configuration
change when current preimages match, while keeping both copies. Refuse unknown
preimages. A whole-prefix restore is an explicit user recovery operation with
save/account/Cloud implications, not a routine Steam rollback. Do not pin or
automatically downgrade Valve's client; it may update again or cease to work
with server-side services. Updates to Wine and module policy are separate.

## SCGL publication boundary

See the [source review](../../modules/simcity-4/docs/investigation.md#source-and-license-review).
Keep the aggregate patch for reproducing the historical investigation. Its
16-file scope is broader than texture slot ordering. An upstream submission
should isolate that ABI correction and its regression check, then independently
test the smaller patch; do not attribute the aggregate result to it.

Source applicability and license identification can be reviewed now. Binary
distribution remains gated on a pinned build recipe, dependency notices and
source obligations, old/new ABI regression results, and fresh attributed game
observations. Rebuilding the same source need not reproduce an old binary hash;
record the new artifact honestly and reverify it.

## Implementation order and proof

1. Fix signal outcomes and plan purity. A signal-terminated child must fail;
   launch/installer/CEF/reset plans must leave synthetic trees byte-identical.
2. Add conservative stage preservation. Failed copy/patch and refresh requests
   must leave sentinel saves and the source unchanged.
3. Add the read-only module catalog independently of runtime safety changes.
   Cover valid SC4, unknown fields/version, duplicate IDs, escaping/symlink
   paths, Steam mismatch, and forbidden public host paths/artifacts.
4. Add managed installation ownership and locking across all mutation callers.
   Use two OS processes for contention tests; test aliases, separate prefixes,
   supervisor death with surviving children, and unsupported locking failure.
5. Add replacement only with save reconciliation and interruption tests at each
   journal/rename boundary. Do not loosen the conservative guard first.
6. Add Steam fingerprint comparison and conditional wrapper recovery after
   ownership is enforced. Test changed CEF, stale backup, unknown identity,
   interrupted update, and mismatch refusal using synthetic files.

CI for existing Rust checks and read-only catalog tests is independent work.
Player packaging, a detached supervisor, and broader ABI correctness require
their own execution review. None is established by this design document.
