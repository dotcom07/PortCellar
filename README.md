# PortCellar

Agent-assisted compatibility engineering for Windows games on macOS.

PortCellar aims to turn individual Wine investigations into reusable game
modules: configuration, patches, reproduction steps, and evidence of exactly
what was tested. The intended workflows cover both Windows Steam games and
separately installed Windows games.

**Current state: experimental runtime and read-only module catalog.** The existing
two-crate runtime is available as `portcellar-cli` and `portcellar-core`. The
SimCity 4 module contains the reviewed profile, SCGL patch, checker, and
investigation notes. Isaac's existing Steam workflow and historical observations
are also documented. There is no turnkey player download yet.

## Start here

- **SimCity 4 players and modders:** start with the [SimCity 4 module](modules/simcity-4/).
  It describes the SCGL texture fix, historical observations, and known limits.
- **Developers:** read the [architecture and migration proposal](docs/architecture/portcellar-plan.md).
- **Isaac:** read the [Isaac module](modules/binding-of-isaac-rebirth/).
- **Windows Steam:** read the [shared client guide](components/steam/).
- **Project background:** read the [original concept](PortCellar.md).

The first public technical result is planned around SimCity 4. Isaac's module
preserves historical results and a later unsuccessful login precondition.
Launching a game, completing a tutorial, and verifying a full playthrough are
different claims.

Players should eventually be able to use reviewed modules without an LLM account
or a compiler. Agents help investigate new failures; the published runtime and
module should preserve the result for everyone else.

## Development

```sh
cargo test --all-targets
cargo run -- --help
cargo run -- game profiles --root modules/simcity-4/profiles
cargo run -- game modules --root modules
cargo run -- game module --root modules --id binding-of-isaac-rebirth
```

Private analysis, runtime, prefix, build, trace, and evidence state is stored
under `.portcellar/` during repository development. Set `PORTCELLAR_STATE_ROOT`
to use a different state directory.

Module commands validate descriptors and referenced profiles; they do not launch
or install a module. Existing direct-profile and Isaac commands remain available.
Launch plans do not materialize game copies. Execution reuses a ready staged copy
and refuses refresh/replacement of an existing copy to preserve saves. Managed
prefix ownership, concurrent mutation guards, and Steam update fingerprinting
remain unfinished; see the [implementation status](docs/architecture/foundation-decisions.md#implementation-status).

PortCellar will not include games, Steam account data, or proprietary CrossOver
components. Runtime packages and game modules will have separate versions and
distribution requirements.
