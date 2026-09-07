# Wine Patch Candidates

These patches are optional engine-side candidates for a custom PortCellar Wine build.

The runtime should prefer prefix-level and launch-level fixes first. Apply these only when a
stock Wine or CrossOver-derived engine cannot reproduce the required behavior.

## Patches

- `steam-macdrv-immovable-default.patch`: narrows Wine mac driver window remapping for
  `steam.exe` and `steamwebhelper.exe`. This is intended for Steam CEF login windows that
  report sentinel/offscreen coordinates on macOS Wine builds.
- `steam-macdrv-offscreen-placement.patch`: recenters small windows that are fully outside
  the desktop after Wine's Cocoa coordinate clamp. This targets Steam CEF login windows that
  are internally ready but have no visible macOS window.
- `steam-cef-flags-idempotent.patch`: keeps CrossOver/GPTK-style Steam CEF command-line
  injection from appending duplicate flags when PortCellar already passes them at
  runtime launch level.
- `simcity4-apple-opengl-compat.patch`: recognizes the Apple Silicon OpenGL extension
  combination used by the M4 renderer, skips the fragile Apple FBO format probe for the
  legacy SC4 path, and uses raw monitor DPI for legacy OpenGL window coordinates. It is
  validated against the Wine 11.13 source tree and is not applied to the prebuilt Wine
  11.10 bundle automatically.
- `simcity4-apple-opengl-compat-wine-11.10.patch`: the matching patch for a pristine Wine
  11.10 worktree. Use it when rebuilding the exact engine version; do not mix its DLLs with
  another Wine build.

The runtime already applies the equivalent registry workaround:

```sh
wine reg add "HKCU\Software\Wine\Mac Driver" /v AllowImmovableWindows /d N /f
```

The patch keeps explicit registry overrides intact and only changes the app-specific default.

The runtime also passes the Steam CEF compatibility flags directly:

```sh
steam.exe -allosarches -cef-force-32bit -no-cef-sandbox
```

That launch-level fix should be tried before building a custom Wine engine.

Apply an engine patch from the repository root only to a dedicated Wine worktree:

```sh
git apply --directory=upstreams/wine runtimes/wine/patches/simcity4-apple-opengl-compat.patch
```

For the exact 11.10 source, create a worktree at the `wine-11.10` tag and apply the
version-specific patch there:

```sh
git -C upstreams/wine worktree add /tmp/portcellar-wine-11.10 refs/tags/wine-11.10
git apply --directory=/tmp/portcellar-wine-11.10 \
  runtimes/wine/patches/simcity4-apple-opengl-compat-wine-11.10.patch
```

The resulting `wined3d`/`winemac` binaries must be installed as one version-matched set.
