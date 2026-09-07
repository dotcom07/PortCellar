# Engineering and growth roadmap

PortCellar is an experimental, two-crate Rust runtime with a read-only module
catalog. The roadmap grows from that foundation while keeping game policy in
modules and shared execution behavior in the runtime. It assumes a solo
maintainer working on Apple Silicon macOS. Dates are intentionally omitted;
each milestone exits on evidence, not on a calendar.

## Near term: make the first result reproducible

### Architecture and documentation audit

- Keep `portcellar-cli` and `portcellar-core` as the current boundary.
- Align README, agent guidance, module contracts, and implementation status with
  the code. Remove stale claims as they are found.
- Preserve the distinction between module, variant, installation, and run
  identity, and between a Steam App ID and a game identity.
- Keep plans read-only and stage handling conservative until ownership controls
  exist. Document manual observations separately from deterministic checks.

Exit when public docs distinguish implemented behavior, selected design, and historical evidence,
with focused tests or explicit, named gaps for selected contracts.

### SimCity 4 source-only publication, in parallel with ownership work

Prepare a reproducible technical release for Windows SimCity 4 1.1.610.0 (x86), standalone, under
the tested Apple Silicon/macOS route. It may ship before the whole platform or a player package is
complete. The release should include the SCGL source and aggregate patch with its full scope,
a pinned or fully recorded build recipe, notices and corresponding source, a checker that fails
explicitly under normal and optimized Python, and sanitized evidence.

Its exit gate is a clean old/new build comparison, exact artifact and input identities, and a fresh
attributed run through the selected launch, region, and Getting Started Tutorial scenarios.
The wording must remain bounded: this is a source-only technical finding, not full SC4 compatibility
or Steam verification. The first post-publication feedback target is two independently reproducible
reports; it is not a prerequisite for the first announcement.

Community sequence: GitHub is canonical, then a technical Simtropolis thread, then Reddit/Discord
feedback after checking each venue's current rules. Drafts are prepared but never posted as part
of this roadmap. Lead with the technical finding and evidence; do not advertise AI or platform access.

### Ownership engineering

Add the smallest core-owned boundary that can enroll installations, lock shared
prefix/stage resources, supervise foreground execution, and preserve failed
operations. Test contention, aliases, surviving children, and unsupported
locking with synthetic inputs. Keep legacy inspection available, but do not
present it as managed safety.

Exit when every mutation path routes through the boundary and tests show busy,
ambiguous, interrupted, and failed operations leave recoverable state.

## Medium term: make reviewed results usable

### Recovery and Steam reverification

Build on the near-term installation ownership and supervision boundary with
replacement journals and save/config reconciliation. Add Steam artifact
fingerprints and invalidation for client, WebHelper, wrapper, runtime, and
profile changes. Reverification must distinguish login-required user action,
client health, and game scenario results.

Exit when synthetic update, stale-identity, recovery, and interruption cases are
covered, and a changed fingerprint cannot silently reuse old evidence.

### Experimental player package

Package the runtime and module as separate versioned artifacts. A player package
must install and update without Rust, MinGW, or a local Wine build; preserve
original files, plugins, saves, and user data; and provide recovery after a
failed update or uninstall. It must state architecture and Rosetta/Wine
requirements, include integrity and license material, and be tested by another
person on a clean eligible host. No package is promised before those checks.

### Grow by technical diversity

Reverify Isaac before making a current compatibility claim within its Windows
Steam scope and known login precondition, independently from standalone SC4.
Add a third distinct game only when its technical behavior adds reusable
coverage, verification is feasible for a solo maintainer, and the module has
its own evidence and limits.
Select games by technical diversity, reuse value, and feasible verification,
not by catalog size.

## Long term: reuse what evidence justifies

Extract reusable module/runtime pieces only when a second consumer or stable
dependency boundary requires them. Build a tested agent-facing CLI over the same
operations humans use, with structured results and explicit user-action states.
Add MCP, a GUI, or search only after demonstrated demand and measured friction
justify the extra surface; none is a prerequisite for the first SC4 release.

## Measures and ownership

Track externally reproduced scenarios and configurations, first-run effort,
maintainer intervention, and regression response time. The maintainer owns the
root roadmap and shared contracts; each module owns its settings, evidence,
limitations, and release notes. Record decisions in the existing architecture
and module documents rather than inventing duplicate frameworks or status
systems.
