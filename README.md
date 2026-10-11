# PortCellar

Agent-assisted compatibility engineering for Windows games on macOS.

PortCellar aims to turn individual Wine investigations into reusable game
modules: configuration, patches, reproduction steps, and evidence of exactly
what was tested. The intended workflows cover both Windows Steam games and
separately installed Windows games.

**Current state: experimental runtime, managed standalone execution, and read-only module catalog.** The existing
two-crate runtime is available as `portcellar-cli` and `portcellar-core`. Game
modules collect profiles, investigation notes, and available patches and checks.
There is no turnkey player download yet.

The [managed standalone route](docs/runtime/managed-installations.md) owns a new
installation's prefix, copied game and recovery data, with foreground execution,
OS locking and exact file-change checks. Existing-prefix enrollment, shared
Steam ownership and detached supervision are not implemented.

## Start here

- **Browse games:** find game-specific instructions and investigation records in
  the [game catalog](modules/README.md).
- **Project direction:** see the [engineering and growth roadmap](docs/roadmap.md).
- **Developers:** read the [architecture and migration proposal](docs/architecture/portcellar-plan.md).
- **Windows Steam:** read the [shared client guide](components/steam/).
- **Project background:** read the [original concept](PortCellar.md).

Launching a game, completing a tutorial, and verifying a full playthrough are
different claims. Each module documents its own observations and limitations;
being listed in the catalog does not establish compatibility.

Players should eventually be able to use reviewed modules without an LLM account
or a compiler. Agents help investigate new failures; the published runtime and
module should preserve the result for everyone else.

## Development

```sh
cargo test --all-targets
cargo run -- --help
cargo run -- game modules --root modules
```

Private analysis, runtime, prefix, build, trace, and evidence state is stored
under `.portcellar/` during repository development. Set `PORTCELLAR_STATE_ROOT`
to use a different state directory.

Module commands validate descriptors and referenced profiles; they do not launch
or install a module. Existing direct-profile and game-specific commands remain available.
Launch plans do not materialize game copies. Execution reuses a ready staged copy
and refuses refresh/replacement of an existing copy to preserve saves. Ownership
and mutation guards cover the managed standalone route. Their broader migration
to legacy/Steam workflows and Steam update fingerprinting remain unfinished;
see the [implementation status](docs/architecture/foundation-decisions.md#implementation-status).

PortCellar will not include games, Steam account data, or proprietary CrossOver
components. Runtime packages and game modules will have separate versions and
distribution requirements.
