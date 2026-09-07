# Game catalog

Find instructions, investigation records, and known limitations for each game.
All current modules are experimental; there is no turnkey player download yet.
A catalog entry means that a module exists, not that the game is compatible.

| Game | Distribution covered | What you will find |
| --- | --- | --- |
| [SimCity 4](simcity-4/README.md) | Windows Deluxe 1.1.610.0, standalone | SCGL texture/ABI investigation, source patch, binary checker, and historical region/tutorial observations. |
| [The Binding of Isaac: Rebirth](binding-of-isaac-rebirth/README.md) | Windows Steam, App ID 250900 | Existing launch workflow, historical gameplay/save observations, and a later failed login precondition. Current compatibility needs reverification. |

Read the linked module before testing: it defines the configuration observed,
what was verified, and what remains unknown. Historical observations are not a
fresh verification of the current game, Steam client, or runtime.

For shared client setup and diagnosis, see the [Windows Steam guide](../components/steam/README.md).

## Inspect modules from source

Run these commands from the repository root:

```sh
cargo run -- game modules --root modules
cargo run -- game module --root modules --id simcity-4
```

These commands validate module descriptors and their referenced profiles. They
do not install or launch games, and their success does not verify gameplay.

## Adding a game

Add the game's descriptor, profiles, and human-facing instructions under its
module directory, then add one entry to this manually maintained catalog.
Keep verification details in the game README and its evidence documents.
Update the root README when project-wide guidance changes; adding a game alone
does not require a root README edit. See the
[module contract](../docs/architecture/foundation-decisions.md#module-descriptor-v1-descriptive-catalog-first)
for descriptor requirements.
