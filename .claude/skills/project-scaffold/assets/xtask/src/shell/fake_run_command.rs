use crate::types::{Captured, CommandLine, RunCommand, SpawnFailed};

/// The in-memory [`RunCommand`]: answers from a table and starts nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeRunCommand {
    responses: Vec<(CommandLine, Result<Captured, SpawnFailed>)>,
}

impl FakeRunCommand {
    /// Makes a fake that answers each listed command line with its paired result.
    ///
    /// # Parameters
    /// - `responses`: the table. When a command line is listed more than once, the first entry
    ///   wins. May be empty.
    #[must_use]
    pub fn new(responses: Vec<(CommandLine, Result<Captured, SpawnFailed>)>) -> Self {
        Self { responses }
    }
}

impl RunCommand for FakeRunCommand {
    /// Returns a clone of the result paired with the first table entry equal to `command_line`
    /// (program and every argument equal, in order).
    ///
    /// # Errors
    /// - [`SpawnFailed`] as listed in the table for that command line.
    /// - [`SpawnFailed`] with the command line's `program` and the reason
    ///   `no fake response for this command line` when the table has no equal entry.
    ///
    /// # Remarks
    /// Pure. The same command line always gets the same answer.
    fn run(&self, command_line: &CommandLine) -> Result<Captured, SpawnFailed> {
        self.responses
            .iter()
            .find(|(listed, _)| listed == command_line)
            .map_or_else(
                || {
                    Err(SpawnFailed {
                        program: command_line.program.clone(),
                        reason: String::from("no fake response for this command line"),
                    })
                },
                |(_, result)| result.clone(),
            )
    }
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "fake_run_command_tests.rs"]
mod tests;
