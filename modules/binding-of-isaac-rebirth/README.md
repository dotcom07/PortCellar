# The Binding of Isaac: Rebirth

**Distribution:** Windows Steam, App ID `250900`, depot `250902`

**Executable:** `isaac-ng.exe`

**Investigation host:** Apple Silicon macOS

**Status:** Experimental — historical gameplay observations; needs reverification

The imported runtime includes the original Isaac launch, login, configuration,
and diagnostic code. There is no packaged game or ready-to-install runtime here.

## For players

Use your own Windows Steam installation and legally owned game in a dedicated
Wine prefix. A live Windows Steam session is required; native macOS Steam login
data and `steam_appid.txt` do not replace it. Keep existing saves backed up.

The current development workflow uses the built-in Isaac commands:

```sh
cargo run -- isaac doctor
cargo run -- isaac play --dry-run
# After selecting your own engine/prefix and reviewing the plan:
cargo run -- isaac play
```

`play` configures Wine and Isaac options, prepares the Steam CEF helper when
needed, starts or reuses Windows Steam, and waits for the game process. Login
still requires user interaction. A stable process is not proof of rendering.
These are experimental legacy execution commands; managed prefix locking and
safe concurrent mutation are not implemented. Use one operation per prefix.

Engine/prefix selection, login troubleshooting, and recovery command behavior
are described in the [shared Steam guide](../../components/steam/README.md).

## Preserved policy and catalog scope

The built-in `isaac-wine-steam-v2` profile selects Windows 10, the
CrossOver-compatible CEF policy, and DXMT configuration
`d3d11.preferredMaxFrameRate=60;`. The policy name does not imply bundled
CrossOver components. Isaac declares OpenGL capability; selecting DXMT does not
prove which rendering API a particular build uses.

The built-in profile writes these runtime options:

| Option | Default |
| --- | --- |
| ControllerHotplug | 0 |
| EnableIntro | 0 |
| EnableMods | 0 |
| Fullscreen | 0 |
| MouseControl | 0 |
| SteamCloud | 0 |
| VSync | 1 |

`PORTCELLAR_ISAAC_STEAM_CLOUD=1` opts into writing `SteamCloud=1`. This setting
alone is not evidence of successful synchronization or a backup of your saves.

The [catalog profile](profiles/windows-steam.toml) records portable launch
metadata. Its `isaac-catalog-v1` revision is intentionally distinct: generic
profiles do not implement the built-in `Save Data Path:` log discovery and
`Binding of Isaac Rebirth` Documents fallback. Consequently it omits runtime
options and makes no parity claim. Use the `isaac` commands for that workflow;
the module catalog only inspects descriptors and profiles.

```sh
cargo run -- game module --root modules --id binding-of-isaac-rebirth
```

## Historical observations and limits

The legacy project's `docs/runtime/isaac.md` reports a Windows Steam launch that
reached gameplay, repeated game-over/restart cycles, and a local save update.
It separately reports a Steam Cloud smoke run that loaded and saved game data.
These are imported maintainer observations, not tests run during this migration.
Exact game/client/binary fingerprints have not been established for a public
reproduction, so they do not establish current compatibility.

The same record's later **2026-07-11** smoke stopped before game launch because
`active_session=false`. Cached credentials and connected transport did not
qualify as an active login. This failed precondition remains part of the record.
Historical native macOS attempts also crashed through the Steam loader/overlay
or `__ARCLite__load()`; this is not a claim about every current native build.

Not yet reverified: current Steam login and CEF, pinned game and runtime builds,
rendering, input/audio, extended sessions, DLC/mod combinations, local save
round trips, and Steam Cloud conflict handling. Report exact versions and the
scenario observed; keep raw logs and account details private.

## Implementation

- [Built-in Isaac policy](../../crates/portcellar-core/src/games/isaac.rs)
- [Version anchors](../../crates/portcellar-core/src/anchors.rs)
- [Shared launch planning](../../crates/portcellar-core/src/runtime/launch.rs)
- [Save/options handling](../../crates/portcellar-core/src/runtime/profile.rs)
