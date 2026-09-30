use crate::types::{StepOutcome, Verdict};

/// Renders the outcomes of a check run as the text shown to the developer.
///
/// # Parameters
/// - `outcomes`: the judged steps, in the order they ran. May be empty.
///
/// # Returns
/// One line per outcome, in order: `ok    <name>` for a pass (`ok`, four spaces, the name) and
/// `FAIL  <name>` for a failure (`FAIL`, two spaces, the name). Each failure line is followed
/// by its detail, one output line per detail line, each indented by four spaces; an empty
/// detail adds no lines, and an empty line inside a detail is kept as four spaces. The last
/// line is `check: <p> passed, <f> failed`. Every line, the last included, ends with `\n`.
/// With no outcomes the result is exactly `check: 0 passed, 0 failed\n`.
///
/// # Remarks
/// Pure. Detail lines are split on `\n`; a `\r` immediately before it is dropped.
#[must_use]
pub fn render_report(outcomes: &[StepOutcome]) -> String {
    let (mut report, passed, failed) = outcomes.iter().fold(
        (String::new(), 0_usize, 0_usize),
        |(mut text, passed, failed), outcome| match &outcome.verdict {
            Verdict::Passed => {
                text.push_str("ok    ");
                text.push_str(&outcome.name);
                text.push('\n');
                (text, passed + 1, failed)
            }
            Verdict::Failed { detail } => {
                text.push_str("FAIL  ");
                text.push_str(&outcome.name);
                text.push('\n');
                if !detail.is_empty() {
                    for line in detail.split('\n') {
                        text.push_str("    ");
                        text.push_str(line.strip_suffix('\r').unwrap_or(line));
                        text.push('\n');
                    }
                }
                (text, passed, failed + 1)
            }
        },
    );

    report.push_str("check: ");
    report.push_str(&passed.to_string());
    report.push_str(" passed, ");
    report.push_str(&failed.to_string());
    report.push_str(" failed\n");
    report
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "render_report_tests.rs"]
mod tests;
