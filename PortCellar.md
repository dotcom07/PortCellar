# Agent-Assisted Windows Game Compatibility Platform for macOS

## 1. Project Definition

This project is an open-source, agent-assisted compatibility engineering platform for running Windows games on macOS.

It combines:

* Wine runtime orchestration
* Windows executable and PE static analysis
* runtime diagnostics
* game-specific configuration
* compatibility patches
* dependency and graphics-backend selection
* reproducible verification
* reusable compatibility modules

The project is not designed around a fixed list of supported games.

Instead, each investigated game becomes an independent compatibility module that can be analyzed, improved, reproduced, and shared without embedding game-specific behavior into the runtime core.

LLM agents may assist with binary analysis, runtime investigation, patch development, configuration, and verification, while deterministic tools remain responsible for modifying runtimes, launching programs, collecting evidence, and producing releases.

The long-term goal is to make Windows game compatibility work on macOS cumulative rather than disposable.

---

# 2. Project Mission

Windows game compatibility work is often fragmented across forum posts, personal Wine prefixes, shell scripts, undocumented registry changes, and one-off patches.

A typical result looks like:

> "Use this Wine build, set these environment variables, copy this DLL, and it seems to work."

That knowledge is difficult to reproduce, validate, maintain, or transfer to another game.

This project instead models compatibility work as:

```text
game
  ↓
static analysis
  ↓
runtime experiment
  ↓
evidence
  ↓
diagnosis
  ↓
profile / patch / dependency change
  ↓
verification
  ↓
compatibility module
```

Each successful investigation adds reusable knowledge to the project.

---

# 3. Compatibility Is Scoped, Not Binary

The project must not classify games simply as:

```text
Works
Does not work
```

Launching an executable does not demonstrate that a game is playable.

Reaching the main menu does not demonstrate that gameplay works.

Completing one tutorial does not demonstrate complete game compatibility.

Every compatibility claim therefore has an explicit verification scope.

## Compatibility states

Suggested states are:

### `analyzed`

Static analysis has been completed.

The project may know:

* executable architecture
* subsystem
* imported DLLs
* graphics APIs
* audio APIs
* middleware
* Steam integration
* launcher structure
* probable runtime requirements

No successful execution is implied.

### `launches`

The main executable starts and survives an initial observation period.

This proves process startup only.

It does not prove correct rendering, input, audio, save behavior, or gameplay.

### `ui-verified`

A meaningful application or game UI has been reached and manually or automatically verified.

Examples:

* title screen
* launcher
* region selector
* main menu

### `scenario-verified`

One or more explicit gameplay scenarios have passed.

For example:

```text
scenario:
  Getting Started Tutorial

result:
  passed

verified:
  city rendering
  terrain textures
  basic camera interaction

not verified:
  long sessions
  save/load
  every renderer path
  every city type
  every plugin
```

### `playthrough-verified`

A substantial gameplay session or defined test suite has been completed.

This still does not imply universal compatibility.

### `broadly-verified`

Multiple users, machines, runtime versions, and gameplay scenarios provide repeatable evidence.

Only modules with substantial evidence should reach this state.

---

# 4. Compatibility Claims

Compatibility status must be based on individual claims rather than a single global boolean.

Example:

```yaml
verification:
  - claim: process-launch
    result: pass

  - claim: region-view-rendering
    result: pass

  - claim: getting-started-tutorial
    result: pass

  - claim: terrain-texture-rendering
    result: pass

  - claim: save-load
    result: unknown

  - claim: long-session-stability
    result: unknown

  - claim: complete-game-compatibility
    result: unknown
```

This distinction is fundamental to the project.

A module may contain valuable fixes even when complete gameplay compatibility has not yet been established.

---

# 5. Core Architecture

The system consists of five major layers.

```text
┌──────────────────────────────────────────────┐
│              Human / LLM Agents              │
│                                              │
│ investigation · diagnosis · patch planning  │
└──────────────────────┬───────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────┐
│              Agent Tool Interface            │
│                                              │
│ inspect · launch · trace · patch · compare   │
│ build · verify · package                     │
└──────────────────────┬───────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────┐
│             Compatibility Engine             │
│                                              │
│ PE analysis                                  │
│ Wine orchestration                           │
│ prefix management                            │
│ process supervision                          │
│ graphics/runtime selection                   │
│ dependency management                        │
│ evidence collection                          │
└───────────────┬────────────────┬─────────────┘
                │                │
                ▼                ▼
┌──────────────────────┐   ┌───────────────────┐
│ Compatibility Module │   │ Runtime Providers │
│                      │   │                   │
│ profiles             │   │ Wine              │
│ patches              │   │ Wine Staging      │
│ tests                │   │ user engines      │
│ dependencies         │   │ future backends   │
│ known issues         │   │                   │
└──────────────────────┘   └───────────────────┘
                │
                ▼
┌──────────────────────────────────────────────┐
│                Evidence Store                │
│                                              │
│ traces · manifests · hashes · logs · tests  │
│ observations · crash data · build metadata  │
└──────────────────────────────────────────────┘
```

---

# 6. Runtime Core

The runtime core must remain game-independent.

Responsibilities include:

* Wine engine discovery
* Wine engine selection
* prefix creation and isolation
* Windows environment preparation
* process ownership
* process supervision
* Windows Steam supervision
* environment variables
* registry configuration
* graphics backend configuration
* runtime dependency installation
* launch-plan generation
* log collection
* crash observation
* stable-runtime observation
* verification command execution

Game names should not appear in core runtime logic.

Instead:

```text
runtime core
        +
compatibility module
        =
game execution plan
```

---

# 7. Compatibility Modules

Every game-specific investigation becomes a compatibility module.

A module may contain:

```text
modules/
  simcity-4/
    module.toml

    profiles/
      base.toml
      wine-opengl.toml

    patches/
      scgl-texture-vtable.patch

    scripts/
      check-scgl-texture-abi.py

    tests/
      launch.toml
      tutorial.toml

    docs/
      investigation.md
      known-issues.md

    evidence-schema/
      texture-abi.toml
```

A compatibility module may define:

* executable selection
* known game versions
* architecture
* runtime requirements
* preferred Wine versions
* graphics backend
* environment variables
* registry values
* required libraries
* DLL overrides
* game-specific patches
* runtime patches
* launch behavior
* expected processes
* validation scenarios
* known failures
* unsupported configurations

Modules should remain declarative wherever possible.

---

# 8. Agent-Native Architecture

LLM agents are first-class users of the project.

This means the repository must not require agents to infer important state from prose.

Important project state should be represented in structured files.

For example:

```yaml
game:
  id: simcity-4
  name: SimCity 4 Deluxe
  executable: SimCity 4.exe

binary:
  architecture: i386
  pe_type: PE32

runtime:
  family: wine-staging
  tested_version: "11.10"

verification:
  highest_level: scenario-verified

claims:
  getting-started-tutorial:
    status: passed

  full-game:
    status: unknown
```

Human-readable Markdown documentation supplements this information rather than replacing it.

---

# 9. Agent Operating Model

Agents should not directly make compatibility claims.

Instead the workflow is:

```text
Agent proposes
      ↓
Tool executes
      ↓
Evidence captured
      ↓
Verifier evaluates
      ↓
Claim updated
```

The agent therefore operates through deterministic capabilities.

Examples:

```text
inspect.pe
inspect.imports
inspect.strings
inspect.resources

runtime.plan
runtime.launch
runtime.stop
runtime.observe

trace.wine
trace.crash
trace.process

patch.apply
patch.revert

build.wine
build.dll

verify.process
verify.binary
verify.scenario

evidence.record
evidence.compare
```

An agent should be able to discover these operations without understanding the Rust implementation.

---

# 10. Agent Workspace

Agent-generated artifacts should remain separate from reviewed project state.

Example:

```text
.portcellar/
  workspaces/
    simcity-4/
      analysis/
      experiments/
      traces/
      builds/
      hypotheses/
      patches/

  evidence/
  prefixes/
  engines/
  downloads/
```

This directory remains ignored by Git.

An agent may experiment freely inside the workspace.

Nothing becomes part of the public compatibility catalog until explicitly promoted.

---

# 11. Promotion Model

The project should distinguish:

```text
experiment
    ↓
candidate
    ↓
reviewed
    ↓
published
```

For example:

```text
.portcellar/workspaces/simcity-4/profile.toml
```

may eventually be promoted to:

```text
modules/simcity-4/profiles/wine-opengl.toml
```

Promotion requires sufficient reproducible evidence.

An LLM agent can prepare the change, but publication should require deterministic checks and optionally human review.

---

# 12. Evidence Model

Evidence is a core project artifact.

Useful evidence includes:

* executable SHA-256
* runtime version
* runtime build hash
* module version
* host architecture
* macOS version
* Wine logs
* exit status
* crash exception
* loaded module list
* debugger traces
* process lifetime
* binary inspection results
* screenshots where available
* manually confirmed observations
* automated regression test results

Evidence should distinguish observation source.

Example:

```yaml
observation:
  claim: tutorial-rendering

  result: pass

  source:
    type: human-confirmation

  automation:
    screenshot: unavailable

  environment:
    host: Apple-Silicon
    runtime: Wine-Staging-11.10
```

This prevents an agent from silently turning an assumption into a verified fact.

---

# 13. Static Analysis Pipeline

Static analysis provides the first stage of a compatibility investigation.

An agent should be able to request:

```text
analyze game/
```

and receive a machine-readable report describing:

* PE executables
* executable architecture
* launcher candidates
* likely game executable
* imported DLLs
* Direct3D/OpenGL usage
* DirectSound/XAudio/OpenAL usage
* video middleware
* Visual C++ runtime dependencies
* Steam API usage
* anti-cheat indicators
* bundled runtime libraries
* configuration files
* probable save locations
* unusual executable characteristics

Static analysis produces hypotheses.

It does not produce compatibility guarantees.

---

# 14. Dynamic Investigation Pipeline

After static analysis:

```text
Static analysis
      ↓
Launch plan
      ↓
Runtime observation
      ↓
Failure classification
      ↓
Hypothesis
      ↓
Controlled experiment
      ↓
Evidence comparison
```

An LLM agent may iteratively perform this loop.

For example:

```text
Crash detected
    ↓
inspect Wine trace
    ↓
identify DLL
    ↓
inspect call site
    ↓
produce patch candidate
    ↓
rebuild
    ↓
rerun exact scenario
    ↓
compare old/new evidence
```

This is the process used by compatibility engineering, expressed as reusable infrastructure.

---

# 15. Regression Tests

A compatibility fix should preferably include a regression test.

Tests may be:

### Binary tests

Example:

```text
Verify that expected virtual functions occupy required DLL slots.
```

### Configuration tests

Example:

```text
Verify required registry values before launch.
```

### Runtime tests

Example:

```text
Process survives 30 seconds after launch.
```

### Scenario tests

Example:

```text
Reach region view.
```

Some scenario verification may require human confirmation.

The test metadata must state this explicitly.

---

# 16. SimCity 4 Example

SimCity 4 demonstrates why scoped verification is required.

Observed environment:

```text
Game:
  SimCity 4 1.1.610.0

Runtime:
  Wine Staging 11.10

Host:
  Apple Silicon

Renderer:
  MinGW-built SCGL
```

The investigation identified a C++ virtual-table ABI mismatch affecting SCGL texture operations.

The fix corrected six texture-related virtual function slots.

Regression testing verified the binary layout.

The previous rendering-related access violations stopped occurring.

The Getting Started Tutorial was successfully executed and visually confirmed by the user.

This establishes:

```text
process launch:
PASS

region menu:
PASS

Getting Started Tutorial:
PASS

affected SCGL texture ABI:
PASS

previous access violation:
NOT REPRODUCED AFTER FIX
```

It does not establish:

```text
complete game compatibility:
UNKNOWN

all rendering paths:
UNKNOWN

all SCGL ABI interfaces:
UNKNOWN

long-session stability:
UNKNOWN

all plugins:
UNKNOWN

all save/load scenarios:
UNKNOWN
```

Additional static ABI differences were observed in other overload groups but were not changed because they were outside the verified failure path.

This is the type of scoped compatibility record the project should preserve.

---

# 17. Repository Architecture

Recommended layout:

```text
project/
├── crates/
│   ├── cli/
│   ├── runtime/
│   ├── analyzer/
│   ├── evidence/
│   ├── module/
│   └── agent-tools/
│
├── modules/
│   ├── simcity-4/
│   ├── binding-of-isaac-rebirth/
│   └── final-fantasy-vi/
│
├── runtimes/
│   ├── manifests/
│   ├── patches/
│   └── recipes/
│
├── schemas/
│   ├── module.schema.json
│   ├── evidence.schema.json
│   ├── verification.schema.json
│   └── experiment.schema.json
│
├── tools/
│   ├── binary/
│   ├── runtime/
│   ├── verification/
│   └── packaging/
│
├── docs/
│   ├── architecture/
│   ├── agent-protocol/
│   ├── compatibility-model/
│   └── contributing/
│
├── scripts/
├── upstreams/
│
└── .portcellar/
    ├── workspaces/
    ├── engines/
    ├── prefixes/
    ├── evidence/
    └── builds/
```

`.portcellar/` remains local and ignored.

Everything outside it should be reproducible, reviewable, and suitable for publication.

---

# 18. Runtime Distribution

The project may distribute open-source Wine runtime builds where their licenses permit it.

Runtime packages should be separate from game compatibility modules.

For example:

```text
Runtime
  wine-staging-11.10-macos

Compatibility modules
  simcity-4
  binding-of-isaac-rebirth
  final-fantasy-vi
```

This avoids coupling individual games to a single runtime.

A module can instead express constraints:

```yaml
runtime:
  compatible:
    - wine-staging >= 11.10

  tested:
    - wine-staging-11.10
```

The distinction between `compatible` and `tested` is important.

Compatibility should never be inferred merely because a version satisfies a numeric constraint.

---

# 19. Game Modules Are Knowledge Packages

A compatibility module should be considered a knowledge package rather than merely a launcher profile.

It may include:

```text
configuration
patches
runtime constraints
diagnostics
known issues
verification scenarios
binary fingerprints
workarounds
regression tests
```

This allows discoveries made while supporting one game to remain reproducible years later.

---

# 20. Cross-Game Reuse

Eventually, individual fixes may move out of game modules into reusable capabilities.

For example:

```text
game-specific discovery
        ↓
multiple games need same fix
        ↓
general compatibility component
```

Possible shared components include:

* Steam CEF compatibility
* old DirectDraw handling
* legacy OpenGL behavior
* codec installation
* 32-bit prefix policies
* launcher bypass logic
* process supervision
* specific Wine patches
* legacy Visual C++ runtimes

The system should therefore support inheritance or composition.

For example:

```yaml
module:
  uses:
    - steam-legacy
    - wine-opengl
    - win32-game
```

Game modules should compose capabilities instead of duplicating scripts.

---

# 21. Role of LLM Agents

LLM agents are intended to reduce the cost of compatibility engineering.

They may:

* inspect static analysis results
* form failure hypotheses
* search source trees
* compare runtime traces
* identify suspicious APIs
* produce configuration experiments
* generate patch candidates
* rebuild open-source dependencies
* write regression tests
* document investigations
* update compatibility modules

They should not be treated as authoritative sources of compatibility state.

The authoritative chain is:

```text
artifact
+
deterministic operation
+
captured evidence
+
explicit verification scope
```

---

# 22. Multi-Agent Support

The architecture should permit different agents to work independently.

Example roles:

```text
Analyzer Agent
  static binaries
  imports
  dependency mapping

Runtime Agent
  Wine configuration
  launch experiments
  prefix management

Debugging Agent
  crashes
  traces
  debugger evidence

Patch Agent
  source modification
  rebuild
  regression tests

Verification Agent
  compares claims against evidence

Documentation Agent
  converts reviewed evidence into public reports
```

These do not need to be permanently separate LLMs.

They are logical roles with separate permissions and responsibilities.

A single agent may perform multiple roles.

---

# 23. Agent Handoff Format

Agent work should be transferable.

Instead of:

```text
"I think changing DXVK fixed it."
```

a handoff should look like:

```yaml
experiment:
  id: exp-0042

hypothesis:
  graphics-backend-causes-launch-failure

change:
  backend: wine-opengl

baseline:
  evidence: evidence/exp-0041.json

result:
  evidence: evidence/exp-0042.json

observation:
  process-survival:
    before: 2.1s
    after: 60.0s

conclusion:
  status: candidate

next:
  - verify-rendering
  - verify-input
```

Another agent can continue from this record without reconstructing the entire conversation.

---

# 24. Safety and Reproducibility Boundaries

Agents should operate inside explicit boundaries.

They should not automatically:

* delete arbitrary Wine prefixes
* modify unrelated system configuration
* overwrite user game installations
* publish proprietary game binaries
* redistribute proprietary runtime components
* upload private logs
* publish Steam credentials or identifiers

Destructive operations should require explicit commands or policy approval.

Runtime experiments should prefer isolated prefixes and reversible changes.

---

# 25. Public Project Positioning

The project should not describe itself as:

> A launcher for several Windows games.

It should instead describe itself as:

> An open-source, agent-assisted Windows game compatibility platform for macOS.

A longer description:

> The project combines Wine runtime orchestration, static binary analysis, runtime diagnostics, compatibility patches, and reusable per-game modules. LLM agents can assist with compatibility investigations while reproducible tools and evidence define what has actually been verified.

The existing games are reference compatibility modules developed while validating the platform.

They demonstrate the architecture.

They do not define its scope.

---

# 26. Suggested GitHub Description

> Agent-assisted Wine compatibility engineering for Windows games on macOS.

Alternative:

> Open-source Windows game compatibility runtime, analysis toolkit, and modular profile system for macOS.

---

# 27. Project Principle

The central design principle is:

> **Agents reason. Tools execute. Evidence verifies. Modules preserve the result.**

The objective is not to claim that every game automatically works.

The objective is to make each compatibility investigation cheaper, more reproducible, and reusable by the next person—or the next agent.

Over time, individual game investigations accumulate into a shared compatibility knowledge base for running Windows games on macOS.
