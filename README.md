# PortCellar

Agent-assisted compatibility engineering for Windows games on macOS.

PortCellar aims to turn individual Wine investigations into reusable game
modules: configuration, patches, reproduction steps, and evidence of exactly
what was tested. The intended workflows cover both Windows Steam games and
separately installed Windows games.

**Current state: foundation imported; first module under review.** The existing
two-crate runtime is available as `portcellar-cli` and `portcellar-core`. The
SimCity 4 module contains the reviewed profile, SCGL patch, checker, and
investigation notes. There is no turnkey player download yet.

## Start here

- **SimCity 4 players and modders:** start with the [SimCity 4 module](modules/simcity-4/).
  It describes the SCGL texture fix, historical observations, and known limits.
- **Developers:** read the [architecture and migration proposal](docs/architecture/portcellar-plan.md).
- **Project background:** read the [original concept](PortCellar.md).

The first public technical result is planned around SimCity 4. Other games will
be listed with their own evidence as their modules are imported and reviewed.
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
```

Private analysis, runtime, prefix, build, trace, and evidence state is stored
under `.portcellar/` during repository development. Set `PORTCELLAR_STATE_ROOT`
to use a different state directory.

PortCellar will not include games, Steam account data, or proprietary CrossOver
components. Runtime packages and game modules will have separate versions and
distribution requirements.
