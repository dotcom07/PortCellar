# Windows Steam on Wine

Shared client behavior belongs in the runtime; game metadata belongs in modules.
The imported implementation already includes login/session inspection, CEF helper
preparation, client diagnostics, launch waiting, and stop/reset commands. Some
shared commands still use the historical `isaac` namespace and its prefix
discovery. A Steam client update can invalidate any earlier observation.

## Select the local installation

Set these to your own paths when discovery is ambiguous:

| Environment variable | Purpose |
| --- | --- |
| `PORTCELLAR_WINE` | Wine executable |
| `PORTCELLAR_WINEPREFIX` | Windows prefix |
| `PORTCELLAR_STEAM_EXE` | Windows Steam executable |
| `PORTCELLAR_DXMT_ROOT` | Optional local DXMT root |
| `PORTCELLAR_STATE_ROOT` | Private PortCellar analysis/evidence state |

Without an explicit Steam path, discovery checks `C:\Steam`,
`C:\Program Files (x86)\Steam`, and `C:\Program Files\Steam` in the selected
prefix. The client and game must be installed by the user. PortCellar ships
neither Valve account data nor proprietary CrossOver binaries.

## Inspect, sign in, launch

```sh
cargo run -- isaac doctor
cargo run -- isaac steam-login --dry-run
cargo run -- isaac steam-login --wait --timeout-seconds 600
cargo run -- isaac launch --mode wine-steam --wait --capture-wine-log
```

Login is interactive. `wine-steam` requests `steam.exe -applaunch 250900` for
Isaac. `wine-direct` targets `isaac-ng.exe` in the same prefix and still needs
an active Windows Steam session. `--wait-login` waits for that session first;
`--wait` checks process/runtime stability, not pixels or gameplay. `isaac play`
combines the existing preparation, login, launch, and observation workflow.

Inspect evidence as separate questions: client alive, WebUI available,
transport connected, current login active, library visible, game launched,
game rendering, and owned processes stopped. Cached credentials, old logon
messages, and a WebUI websocket connection are not a current authenticated
session. A CoreGraphics window count is not proof of a visible login screen.

## CEF and client diagnosis

Steam uses `steamwebhelper.exe` for its Chromium UI. Historical investigations
found black login surfaces, restart loops, and orphan WebHelpers after Steam
exited. The runtime distinguishes main-client and helper liveness. It also
provides these existing commands:

```sh
cargo run -- isaac steam-patch-cef --dry-run
cargo run -- analyze steam-client
```

`analyze steam-client` writes a private diagnostic report. `--probe` additionally
starts a client probe; `--stop-after` requests cleanup. Review and sanitize any
report before sharing it: ignored state and summarized account fields do not
make all paths or logs publication-safe.

CEF preparation builds a MinGW wrapper and retains the original helper as
`steamwebhelper_real.exe`. Login and Wine Steam launch may apply it automatically.
The Isaac policy uses the existing CrossOver-compatible GPU/ANGLE flags. Steam
can overwrite that wrapper during updates. Marker-based detection currently
does not establish exact original/wrapper identity across client versions.

`--legacy-login` enables the older `-noreactlogin` experiment.
`PORTCELLAR_STEAM_CEF_SINGLE_PROCESS=1` enables an optional CEF fallback that may
increase CPU use; it is not the default. `PORTCELLAR_STEAM_ARGS` adds
whitespace-separated client arguments. None of these bypasses authentication.

## Stop and recovery limits

```sh
cargo run -- isaac steam-stop --dry-run
cargo run -- isaac steam-reset-session --dry-run
```

The non-dry-run stop path calls the selected prefix's `wineserver -k` and may
terminate residual processes. Reset moves selected session/cache/log targets
to `portcellar-backups` under Steam; it retains `steamapps` and `config.vdf`.
It can require another login. Inspect the proposed targets before using it.

Legacy detached launches create a Unix process group, but the CLI does not hold
a managed resource lock for the entire Wine session. Existing process attribution
and group cleanup are not verified isolation boundaries. Do not run preparation,
reset, patching, or other mutation concurrently against a live/shared prefix.
Wine prefixes are configuration environments, not security sandboxes.

The next [execution contract](../../docs/architecture/foundation-decisions.md)
requires prefix ownership/locking before conditional wrapper recovery. It also
requires separate SHA-256 identities for Steam, original CEF, wrapper, runtime,
and profile; changed or missing identities must require reverification. These
guards are not implemented yet. There is no automatic client downgrade policy.

## Reverification checklist

Record client build/channel, game build, Wine/macOS/architecture, selected
profile, and artifact identities when available. Missing values remain unknown.
Check client startup, WebUI, interactive login, library, an owned installed game,
and shutdown separately after updates. Preserve prior evidence as historical.
For Isaac-specific observations, including a later failed login precondition,
see the [Isaac module](../../modules/binding-of-isaac-rebirth/README.md).

- [Steam inspection](../../crates/portcellar-core/src/analyze/steam_client.rs)
- [CEF implementation](../../crates/portcellar-core/src/runtime/steam_cef.rs)
- [Session reset implementation](../../crates/portcellar-core/src/runtime/session.rs)
- [Valve command-line reference](https://developer.valvesoftware.com/wiki/Command_line_options_(Steam))
