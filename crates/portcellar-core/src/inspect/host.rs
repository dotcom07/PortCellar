use super::*;
use crate::*;
use std::env;
use std::process::Command;

pub(crate) fn inspect_host() -> HostReport {
    HostReport {
        native_arch: command_stdout("/usr/bin/arch", &[])
            .unwrap_or_else(|| env::consts::ARCH.into()),
        translated: sysctl_proc_translated(),
        rosetta_x86_64: Command::new("/usr/bin/arch")
            .arg("-x86_64")
            .arg("/usr/bin/true")
            .status()
            .map(|status| status.success())
            .unwrap_or(false),
    }
}

pub(crate) fn sysctl_proc_translated() -> Option<bool> {
    command_stdout(
        "/usr/sbin/sysctl",
        &["-in".into(), "sysctl.proc_translated".into()],
    )
    .map(|value| value.trim() == "1")
}
