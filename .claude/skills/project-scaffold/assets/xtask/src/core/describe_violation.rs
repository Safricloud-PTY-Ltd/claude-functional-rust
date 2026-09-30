use crate::types::PurityViolation;

/// Describes a purity violation in one line a developer can act on.
///
/// # Parameters
/// - `violation`: the violation to describe.
///
/// # Returns
/// For [`PurityViolation::MissingKind`]:
/// `<package>: no [package.metadata.standards] kind (core, adapter, app or tool)`.
/// For [`PurityViolation::ImpureDependency`]:
/// `<package>: core crate depends on <dependency>, which is not a workspace core crate`.
/// No trailing newline. Names are inserted as they are, without quoting.
///
/// # Remarks
/// Pure.
#[must_use]
pub fn describe_violation(violation: &PurityViolation) -> String {
    match violation {
        PurityViolation::MissingKind { package } => {
            format!("{package}: no [package.metadata.standards] kind (core, adapter, app or tool)")
        }
        PurityViolation::ImpureDependency {
            package,
            dependency,
        } => {
            format!(
                "{package}: core crate depends on {dependency}, which is not a workspace core crate"
            )
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "describe_violation_tests.rs"]
mod tests;
