use std::env;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const COMMAND_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn present_missing(value: bool) -> &'static str {
    if value {
        "present"
    } else {
        "missing"
    }
}

pub(crate) fn find_in_path(binary: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|path| path.join(binary))
        .find(|path| path.exists())
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

pub(crate) fn command_stdout(program: &str, args: &[OsString]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);
    let output = command_output_with_timeout(&mut command)?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn command_stdout_path(program: &Path, args: &[OsString]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);
    let output = command_output_with_timeout(&mut command)?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn command_stdout_stdin(
    program: &str,
    args: &[OsString],
    stdin: &str,
) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    configure_probe_command(&mut command);
    let mut child = command.spawn().ok()?;
    {
        use std::io::Write;
        let child_stdin = child.stdin.as_mut()?;
        child_stdin.write_all(stdin.as_bytes()).ok()?;
    }
    drop(child.stdin.take());
    let output = wait_with_timeout(child)?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn command_output_with_timeout(command: &mut Command) -> Option<Output> {
    configure_probe_command(command);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    wait_with_timeout(command.spawn().ok()?)
}

fn wait_with_timeout(mut child: Child) -> Option<Output> {
    let stdout = child.stdout.take().map(read_pipe_async);
    let stderr = child.stderr.take().map(read_pipe_async);
    let deadline = Instant::now() + COMMAND_PROBE_TIMEOUT;
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), false),
            Ok(None) if Instant::now() >= deadline => {
                terminate_probe_group(&mut child);
                break (child.wait().ok(), true);
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                terminate_probe_group(&mut child);
                break (child.wait().ok(), true);
            }
        }
    };

    let output = Output {
        status: status?,
        stdout: join_pipe_reader(stdout),
        stderr: join_pipe_reader(stderr),
    };
    (!timed_out).then_some(output)
}

fn read_pipe_async<R>(mut reader: R) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut output = Vec::new();
        let _ = reader.read_to_end(&mut output);
        output
    })
}

fn join_pipe_reader(reader: Option<JoinHandle<Vec<u8>>>) -> Vec<u8> {
    reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default()
}

#[cfg(unix)]
fn configure_probe_command(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_probe_command(_command: &mut Command) {}

fn terminate_probe_group(child: &mut Child) {
    #[cfg(unix)]
    {
        let group = format!("-{}", child.id());
        let _ = Command::new("/bin/kill")
            .args(["-KILL", group.as_str()])
            .status();
    }
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_probe_reads_successful_output() {
        let mut command = Command::new("/usr/bin/printf");
        command.arg("probe-ok");
        let output = command_output_with_timeout(&mut command).unwrap();

        assert!(output.status.success());
        assert_eq!(output.stdout, b"probe-ok");
    }

    #[test]
    fn command_probe_drains_output_larger_than_a_pipe_buffer() {
        let mut command = Command::new("/usr/bin/seq");
        command.args(["1", "20000"]);
        let output = command_output_with_timeout(&mut command).unwrap();

        assert!(output.status.success());
        assert!(output.stdout.len() > 100_000);
    }

    #[test]
    fn command_probe_terminates_a_hanging_child() {
        let started = Instant::now();
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "while :; do :; done"]);

        assert!(command_output_with_timeout(&mut command).is_none());
        assert!(started.elapsed() < Duration::from_secs(4));
    }
}
