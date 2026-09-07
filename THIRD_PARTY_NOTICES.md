# Third-party notices

PortCellar links to the following crates at build time:

| Dependency | License source |
| --- | --- |
| `serde` | <https://github.com/serde-rs/serde/blob/master/LICENSE-APACHE> and <https://github.com/serde-rs/serde/blob/master/LICENSE-MIT> |
| `toml` | <https://github.com/toml-rs/toml/blob/main/LICENSE> |

The repository also references pinned upstream source projects under
`upstreams/`. Each upstream keeps its own license and notice files. Those
licenses apply to the corresponding source and any derived binaries; they are
not replaced by the PortCellar license.

The Steam WebHelper wrapper retains its source-specific notice at
`crates/portcellar-core/assets/steamwebhelper-wrapper.LICENSE`.

The SCGL-derived patch under `modules/simcity-4/patches/scgl/` is governed by
LGPL-2.1-or-later, as stated in the upstream source headers at revision
`dc80faec59980da7436e792171e3ce55778f41cd`. Copyright (C) 2025 Nelson Gomez
(nsgomez). Its [license text](modules/simcity-4/patches/scgl/LICENSE) is retained
alongside the patch. PortCellar's MIT/Apache choice does not replace these terms.
Binary distribution must include the applicable notices and corresponding
modified source and build material; see the
[provenance review](modules/simcity-4/docs/investigation.md#source-and-license-review).

PortCellar does not distribute Windows game files, Steam account data, Wine
engines, or other proprietary runtime components in source control.
