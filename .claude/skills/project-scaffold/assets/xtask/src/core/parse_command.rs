use crate::types::{Command, UnknownCommand};

/// Decides which command the arguments ask for.
///
/// # Parameters
/// - `args`: the process arguments after the program name. Only the first is examined; any
///   further arguments are ignored. May be empty.
///
/// # Returns
/// [`Command::Check`] when the first argument is exactly `check`.
///
/// # Errors
/// - [`UnknownCommand`] with `given: None` when `args` is empty.
/// - [`UnknownCommand`] with `given: Some(first)` when the first argument is anything else,
///   including `Check`, ` check` or the empty string: matching is exact and case-sensitive.
///
/// # Remarks
/// Pure.
pub fn parse_command(args: &[String]) -> Result<Command, UnknownCommand> {
    match args.first() {
        Some(first) if first == "check" => Ok(Command::Check),
        Some(first) => Err(UnknownCommand {
            given: Some(first.clone()),
        }),
        None => Err(UnknownCommand { given: None }),
    }
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "parse_command_tests.rs"]
mod tests;
