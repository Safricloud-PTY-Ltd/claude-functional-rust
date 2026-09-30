use crate::core::{describe_violation, find_purity_violations, parse_metadata};
use crate::types::{Captured, Expectation, SpawnFailed, Step, StepOutcome, Verdict};

/// Judges whether a step passed, from what running it produced.
///
/// # Parameters
/// - `step`: the step that was run. Its `name` becomes the outcome's name; its `expectation`
///   selects the rule below.
/// - `result`: what the [`RunCommand`] port returned for the step's command line.
///
/// [`RunCommand`]: crate::types::RunCommand
///
/// # Returns
/// A [`StepOutcome`] named after the step. In the rules below, "the output" means `stdout` and
/// `stderr`, in that order, each with trailing whitespace removed, those that are non-empty
/// after that removal joined by `\n` (the private `join_output`).
/// - `Err(SpawnFailed)`, under any expectation: failed, the detail being the error's `Display`
///   text.
/// - `ExitZero`: passed when `exit_code` is `Some(0)`; otherwise failed with the output.
/// - `NoMatches`: passed when `exit_code` is `Some(1)`. When it is `Some(0)`, failed with
///   `stdout`, trailing whitespace removed (the matches). Anything else, failed with the
///   output.
/// - `PureCoreCrates`: when `exit_code` is not `Some(0)`, failed with the output. Otherwise
///   `stdout` is parsed with `parse_metadata`: on error, failed with that error's `Display`
///   text; on success, passed when `find_purity_violations` finds nothing, otherwise failed
///   with one `describe_violation` line per violation, in order, joined by `\n`.
///
/// # Remarks
/// Pure. Total: every input gets an outcome.
#[must_use]
pub fn judge_step(step: &Step, result: Result<Captured, SpawnFailed>) -> StepOutcome {
    let verdict = match result {
        Err(error) => Verdict::Failed {
            detail: error.to_string(),
        },
        Ok(captured) => match step.expectation {
            Expectation::ExitZero if captured.exit_code == Some(0) => Verdict::Passed,
            Expectation::NoMatches if captured.exit_code == Some(1) => Verdict::Passed,
            Expectation::NoMatches if captured.exit_code == Some(0) => Verdict::Failed {
                detail: captured.stdout.trim_end().to_owned(),
            },
            Expectation::ExitZero | Expectation::NoMatches => Verdict::Failed {
                detail: join_output(&captured),
            },
            Expectation::PureCoreCrates if captured.exit_code != Some(0) => Verdict::Failed {
                detail: join_output(&captured),
            },
            Expectation::PureCoreCrates => match parse_metadata(&captured.stdout) {
                Err(error) => Verdict::Failed {
                    detail: error.to_string(),
                },
                Ok(packages) => match find_purity_violations(&packages) {
                    violations if violations.is_empty() => Verdict::Passed,
                    violations => Verdict::Failed {
                        detail: violations
                            .iter()
                            .map(describe_violation)
                            .collect::<Vec<String>>()
                            .join("\n"),
                    },
                },
            },
        },
    };

    StepOutcome {
        name: step.name.clone(),
        verdict,
    }
}

/// Joins what a process wrote into the one text shown for a failed step.
///
/// # Parameters
/// - `captured`: the finished process. Its exit code is not looked at.
///
/// # Returns
/// `stdout` then `stderr`, each with trailing whitespace removed, those that are non-empty
/// after that removal joined by `\n`. Leading whitespace is kept. Both empty gives the empty
/// string.
fn join_output(captured: &Captured) -> String {
    [captured.stdout.as_str(), captured.stderr.as_str()]
        .into_iter()
        .map(str::trim_end)
        .filter(|text| !text.is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "judge_step_tests.rs"]
mod tests;
