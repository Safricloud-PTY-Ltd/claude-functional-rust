use crate::types::{CrateKind, DependencyScope, Package, PurityViolation};

/// Finds every way the workspace's packages break the crate-purity rule.
///
/// The rule: every package declares a kind, and a package of kind [`CrateKind::Core`] has no
/// runtime dependency other than workspace packages that are themselves of kind core.
///
/// [`CrateKind::Core`]: crate::types::CrateKind::Core
///
/// # Parameters
/// - `packages`: every package in the workspace. A dependency is a workspace package exactly
///   when its name equals the name of an element of this slice. May be empty.
///
/// # Returns
/// The violations, grouped by package in the order of `packages`. Within a package:
/// a [`PurityViolation::MissingKind`] when its `kind` is `None`, and nothing else for that
/// package; otherwise, for a core package, one [`PurityViolation::ImpureDependency`] per
/// offending runtime dependency, in dependency order. A runtime dependency offends when it is
/// not a workspace package, or is a workspace package whose kind is anything but core
/// (including `None`). Dev and build dependencies never offend. Packages of kind adapter, app
/// and tool may depend on anything. An empty vector means the workspace is pure.
///
/// # Remarks
/// Pure. Total: there is no input it rejects.
#[must_use]
pub fn find_purity_violations(packages: &[Package]) -> Vec<PurityViolation> {
    packages
        .iter()
        .flat_map(|package| match package.kind {
            None => vec![PurityViolation::MissingKind {
                package: package.name.clone(),
            }],
            Some(CrateKind::Core) => package
                .dependencies
                .iter()
                .filter(|dependency| {
                    dependency.scope == DependencyScope::Runtime
                        && !packages.iter().any(|candidate| {
                            candidate.name == dependency.name
                                && candidate.kind == Some(CrateKind::Core)
                        })
                })
                .map(|dependency| PurityViolation::ImpureDependency {
                    package: package.name.clone(),
                    dependency: dependency.name.clone(),
                })
                .collect(),
            Some(CrateKind::Adapter | CrateKind::App | CrateKind::Tool) => Vec::new(),
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "find_purity_violations_tests.rs"]
mod tests;
