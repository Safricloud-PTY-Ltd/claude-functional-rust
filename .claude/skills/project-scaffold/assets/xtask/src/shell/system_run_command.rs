use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::types::{Captured, CommandLine, RunCommand, SpawnFailed};

/// The real [`RunCommand`]: starts operating-system processes in a fixed working directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemRunCommand {
    working_dir: PathBuf,
}

impl SystemRunCommand {
    /// Makes a runner whose processes all start in `working_dir`.
    ///
    /// # Parameters
    /// - `working_dir`: the directory each process starts in. It is not checked here; a
    ///   directory that does not exist surfaces as [`SpawnFailed`] when a command is run.
    #[must_use]
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }
}

impl RunCommand for SystemRunCommand {
    /// Starts the program with its arguments in the working directory, with standard input
    /// closed, waits for it to exit, and captures its exit code, standard output and standard
    /// error (each decoded lossily as UTF-8).
    ///
    /// # Errors
    /// - [`SpawnFailed`] when the program cannot be started (not found, not executable, working
    ///   directory missing). `program` is the command line's program; `reason` is the operating
    ///   system error's text and is never empty.
    ///
    /// # Remarks
    /// No shell is involved: arguments reach the program exactly as given. A non-zero exit is
    /// an `Ok`.
    fn run(&self, command_line: &CommandLine) -> Result<Captured, SpawnFailed> {
        Command::new(&command_line.program)
            .args(&command_line.args)
            .current_dir(&self.working_dir)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| SpawnFailed {
                program: command_line.program.clone(),
                reason: error.to_string(),
            })
            .map(|output| Captured {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
    }
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "system_run_command_tests.rs"]
mod tests;
