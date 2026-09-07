# PortCellar Foundation Implementation Plan

> **For agentic workers:** Use this plan task-by-task. Stop at the escalation boundaries below.

**Goal:** Import the reviewed lite-crossover runtime into PortCellar as a clean,
English, game-independent foundation and prepare SimCity 4 as its first public
compatibility module.

**Architecture:** Preserve the existing CLI/core boundary first. Move game policy
into reviewed module data, keep private installations and run artifacts under
`.portcellar/`, and expose deterministic plans, execution, observation, and
evidence. Extract more crates only when a second stable consumer requires it.

**Tech Stack:** Rust 2021, Cargo, existing `serde`/TOML model, Python 3 checker,
Wine-compatible engines, Git submodules for selected upstream source.

**Spec:** `docs/architecture/portcellar-plan.md`; release scope:
`docs/releases/simcity-4-first-public-release.md`.

## Global Constraints

- First-party source and documentation are English; preserve third-party notices.
- Do not change `lite-crossover`, its Git history, or repository visibility.
- Do not import private paths, prefixes, engines, account state, raw logs, game files, or old Git history.
- Use `MIT OR Apache-2.0` for first-party PortCellar code; retain dependency licenses separately.
- Keep `dotcom07` as the public identity; do not publish private identity metadata.
- Start with `portcellar-cli` and `portcellar-core`; do not create empty crates.
- Treat Steam App IDs as namespaced store metadata, not universal game identity.
- Preserve original game files and saves; state-changing work must be scoped and recoverable.
- Do not call a process launch, binary check, or one scenario a full compatibility claim.

### Task 1: Freeze the import boundary

**Files:**

- Create: `.portcellar/migration/import-manifest.toml` (ignored working state)
- Read: the legacy lite-crossover worktree's tracked files, working tree, and submodule pins
- Modify: none in the legacy repository

**Produces:** A manifest listing every candidate source path, destination, origin
revision, treatment (`import`, `translate`, `exclude`, `review`), and reason.

- [ ] Record the legacy commit, index, untracked implementation files, submodule
  revisions, and working-tree diffs.
- [ ] Import only reviewed first-party source/tests, selected scripts/docs,
  required upstream gitlinks, SC4 materials, and legal notices.
- [ ] Exclude local diagnostics, machine/career notes, private profiles, raw
  logs, binaries, game files, engines, prefixes, credentials, and Git history.
- [ ] Verify the manifest has no absolute personal paths, account data, or Korean
  first-party text left as an intended public file.

**Escalate:** Any uncertain file provenance, license, patch scope, or choice to
preserve history. Do not guess.

### Task 2: Import the foundation

**Files:**

- Create: `Cargo.toml`, `Cargo.lock`, `crates/portcellar-cli/`, `crates/portcellar-core/`, selected `docs/`, `scripts/`, `upstreams/`
- Create: `LICENSE`, `THIRD_PARTY_NOTICES.md`
- Modify: `.gitignore`, imported manifests, imported source paths

**Produces:** A clean PortCellar source tree with the existing two-crate runtime
behavior preserved and a complete provenance record.

- [ ] Initialize the new repository when Git metadata is writable and create a
  foundation branch; do not push or change remote visibility.
- [ ] Copy only manifest-approved files. Rename package, binary, public module,
  and environment identifiers consistently.
- [ ] Keep upstreams as pinned source references; do not vendor runtime binaries.
- [ ] Choose and add the first-party MIT/Apache license pair, then inventory all
  imported dependency notices before any binary distribution.
- [ ] Replace owner-specific test fixtures with synthetic paths and preserve
  Unicode/spaces coverage.

**Proof:** `cargo test --all-targets` from the imported workspace; CLI help and
all dry-run commands complete without filesystem mutation.

**Escalate:** Changes to public identity, license interpretation, runtime
ownership, or behavior that requires more than mechanical renaming.

### Task 3: Move private state and neutralize ownership

**Files:**

- Modify: imported root discovery, environment constants, runtime state paths,
  CLI help/examples, tests
- Create: `.portcellar/` state-root fixtures only when a test needs them

**Produces:** Runtime behavior that resolves project-local development state and
installed-app state without relying on an `.portcellar/` directory or a Git checkout.

- [ ] Add a state-root resolver with an explicit environment override, project
  development default, and macOS Application Support default for installed use.
- [ ] Keep prefixes, engines, builds, traces, evidence, and mutable workspaces
  out of tracked files.
- [ ] Preserve plan-only behavior; separate plan creation from staging,
  installation, registry writes, and launch where current code conflates them.
- [ ] Add tests for synthetic home paths containing spaces and Unicode, safe
  relative paths, and no-write dry runs.
- [ ] Add prefix-scoped locking before concurrent mutation or stage refresh, and
  preserve saves while activating a replacement stage.

**Escalate:** Prefix ownership, save locations, signal/timeout semantics, or
  changes affecting multiple runtime callers.

### Task 4: Promote SimCity 4 into a public module

**Files:**

- Create: `modules/simcity-4/README.md`, `module.toml`, `profiles/`, `tests/`, `evidence/`, `docs/investigation.md`
- Move reviewed SCGL patch and checker into `modules/simcity-4/patches/scgl/` and `tools/`
- Modify: module loader/profile types only where required by the declared contract

**Produces:** A standalone SC4 module whose claims are tied to SimCity 4
1.1.610.0, the selected SCGL artifact, runtime fingerprint, and named scenarios.

- [ ] Define module ID, variant ID, installation binding, and run ID separately;
  keep Steam App ID optional and namespaced.
- [ ] Document the six affected vtable entries and the aggregate patch contents;
  do not describe prerequisite changes as only the six-slot fix.
- [ ] Replace checker `assert` calls with explicit failures and test old/new DLLs
  in normal and optimized Python, including unsupported inputs.
- [ ] Record binary SHA-256, compiler, CMake, upstream revision, patch digest,
  runtime components, and observation method in sanitized evidence.
- [ ] Re-run the exact launch, region, and tutorial scenarios only when the
  required runtime and game files are available; preserve human observations as
  human observations.

**Proof:** Old DLL rejection and corrected DLL acceptance are deterministic
binary checks; gameplay claims require a fresh attributed scenario run.

**Escalate:** Any broader ABI fix, Steam SC4 claim, proprietary redistribution,
or change from source publication to turnkey package.

### Task 5: Release and Steam maintenance gates

**Files:**

- Modify: root README, module README, runtime manifests, CI workflow
- Create: release manifest/checksums and reviewed public evidence export

**Produces:** A source-only SC4 technical release first; a player package only
after clean-host, licensing, signing, relocation, update, and rollback checks.

- [ ] Verify fresh checkout reconstruction, formatting, tests, checker, notices,
  and privacy/publication scans.
- [ ] Keep runtime, module, and third-party components separately versioned.
- [ ] Add Steam client/helper fingerprints and update detection; recheck an
  established Steam game after client changes.
- [ ] Test login requiring user action, active session, launch, restart, wrapper
  replacement, and prefix-scoped shutdown without publishing credentials.
- [ ] Publish only after the owner explicitly performs the external release or
  community-post action.

**Escalate:** Signing/notarization, binary hosting, public release, visibility
change, upstream pull request, or community communication.
