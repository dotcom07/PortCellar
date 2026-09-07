use crate::*;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

impl CommandPlan {
    pub fn display(&self) -> String {
        let mut parts = Vec::with_capacity(1 + self.args.len());
        parts.push(shell_quote(
            self.program.as_os_str().to_string_lossy().as_ref(),
        ));
        parts.extend(
            self.args
                .iter()
                .map(|arg| shell_quote(arg.to_string_lossy().as_ref())),
        );
        let command = parts.join(" ");
        let command = if self.envs.is_empty() {
            command
        } else {
            let envs = self
                .envs
                .iter()
                .map(|(key, value)| format!("{}={}", shell_quote(key), shell_quote(value)))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{envs} {command}")
        };

        if let Some(current_dir) = &self.current_dir {
            format!(
                "cd {} && {command}",
                shell_quote(&current_dir.display().to_string())
            )
        } else {
            command
        }
    }
}

pub fn run_plan(plan: &CommandPlan) -> Result<i32> {
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    if let Some(current_dir) = &plan.current_dir {
        command.current_dir(current_dir);
    }
    for (key, value) in &plan.envs {
        command.env(key, value);
    }

    let status = command.status()?;
    status.code().ok_or_else(|| {
        #[cfg(unix)]
        if let Some(signal) = status.signal() {
            return PortCellarError::Message(format!("command terminated by signal {signal}"));
        }

        PortCellarError::Message("command terminated without an exit code".to_string())
    })
}

pub fn run_plan_detached(plan: &CommandPlan) -> Result<u32> {
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    if let Some(current_dir) = &plan.current_dir {
        command.current_dir(current_dir);
    }
    for (key, value) in &plan.envs {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    #[cfg(unix)]
    command.process_group(0);

    let child = command.spawn()?;
    Ok(child.id())
}

pub fn run_plan_detached_with_log(plan: &CommandPlan, log_path: &Path) -> Result<u32> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let stderr = stdout.try_clone()?;

    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    if let Some(current_dir) = &plan.current_dir {
        command.current_dir(current_dir);
    }
    for (key, value) in &plan.envs {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command.stdout(stdout);
    command.stderr(stderr);
    #[cfg(unix)]
    command.process_group(0);

    let child = command.spawn()?;
    Ok(child.id())
}

pub(crate) fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_' | ':' | '='))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::run_plan;
    use crate::CommandPlan;
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn shell_plan(script: &str) -> CommandPlan {
        #[cfg(unix)]
        let (program, prefix) = ("/bin/sh", "-c");
        #[cfg(windows)]
        let (program, prefix) = ("cmd.exe", "/C");

        CommandPlan {
            program: PathBuf::from(program),
            args: vec![OsString::from(prefix), OsString::from(script)],
            envs: BTreeMap::new(),
            current_dir: None,
        }
    }

    #[test]
    fn run_plan_preserves_numeric_exit_codes() {
        assert_eq!(run_plan(&shell_plan("exit 7")).unwrap(), 7);
    }

    #[cfg(unix)]
    #[test]
    fn run_plan_rejects_signal_termination() {
        let error = run_plan(&shell_plan("kill -TERM $$")).unwrap_err();

        assert!(error.to_string().contains("signal 15"));
    }
}
