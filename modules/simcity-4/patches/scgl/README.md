# SCGL Patch Set

`upstreams/scgl` is the pinned upstream source. The game-specific changes are
kept as a parent-repository patch so a fresh clone can reconstruct the same
artifact without committing changes to the upstream repository.

Apply the patch from the repository root after initializing submodules:

```sh
git submodule update --init upstreams/scgl
git apply --directory=upstreams/scgl modules/simcity-4/patches/scgl/simcity4-1.1.610-abi.patch
```

The patch selects the SimCity 4 1.1.610 vtable contract, adds the legacy mode
fallback, selects a legacy WGL context on Apple renderers, and contains the SCGL
diagnostics used during compatibility testing. Diagnostics remain disabled in the
release CMake profile.

The FBO-backed buffer-region path explicitly selects `GL_COLOR_ATTACHMENT0` for
color regions, `GL_NONE` for depth-only FBO completeness, and restores the default
double-buffered `GL_BACK` read/draw state after every region operation. This is
required on Wine's macOS OpenGL compatibility layer; leaving the depth FBO's
`GL_NONE` state active can produce black frames.

The 1.1.610 mode table reports the two fixed-function texture stages implemented
by SCGL and preserves the reference driver's reserved capability flags. Reporting
only one stage can leave the region view working while breaking the multi-texture
terrain path used after selecting a new city.

`SCGL_DISABLE_BUFFER_REGIONS=1` disables the capability and direct region creation
together, so it remains a diagnostic A/B switch rather than a release profile policy.

The release artifact is the default runtime artifact. The diagnostic artifact is
only for collecting `C:\\temp\\scgl-init.log`, including buffer-region blit errors
and bounded pre-swap back/post-swap front pixel samples. A clean-prefix 1600x900
smoke run reached a 289/289 non-black region-selection frame with zero black row or
column masks in both buffers; this is evidence for the repaired state contract, not
a claim that every resolution and scene is covered. The diagnostic artifact must
not be staged into a normal game profile.
