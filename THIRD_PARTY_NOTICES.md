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

PortCellar does not distribute Windows game files, Steam account data, Wine
engines, or other proprietary runtime components in source control.
