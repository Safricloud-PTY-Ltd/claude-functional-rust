//! The whole of `cargo xtask`, as a function of its arguments and the process port.

use crate::core::{check_steps, judge_step, parse_command, render_report};
use crate::types::{Command, RunCommand, RunOutput, StepOutcome, Verdict};

/// Runs `cargo xtask` with the given arguments.
///
/// # Parameters
/// - `args`: the process arguments after the program name, as `parse_command` expects them.
/// - `runner`: the port used to start each step's process.
///
/// # Returns
/// - When `parse_command` fails: exit code 2 and a two-line report, the error's `Display` text
///   then `usage: cargo xtask check`, each line ending in `\n`. The runner is not called.
/// - For [`Command::Check`]: every step of `check_steps`, in order, is run through `runner`
///   and judged with `judge_step`; a failed step does not stop the ones after it. The report
///   is `render_report` of the outcomes. The exit code is 0 when every step passed and 1
///   otherwise.
///
/// [`Command::Check`]: crate::types::Command::Check
///
/// # Remarks
/// Its only effects are the calls it makes on `runner`: exactly one per step, in step order.
#[must_use]
pub fn run<R: RunCommand>(args: &[String], runner: &R) -> RunOutput {
    match parse_command(args) {
        Err(error) => RunOutput {
            exit_code: 2,
            report: format!("{error}\nusage: cargo xtask check\n"),
        },
        Ok(Command::Check) => {
            let outcomes: Vec<StepOutcome> = check_steps()
                .into_iter()
                .map(|step| {
                    let result = runner.run(&step.command_line);
                    judge_step(&step, result)
                })
                .collect();
            let exit_code = u8::from(
                !outcomes
                    .iter()
                    .all(|outcome| outcome.verdict == Verdict::Passed),
            );

            RunOutput {
                exit_code,
                report: render_report(&outcomes),
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "run_tests.rs"]
mod tests;
