//! Contract tests for `describe_violation`.

use proptest::prelude::*;

use super::describe_violation;
use crate::types::PurityViolation;

#[test]
fn describes_missing_kind_with_the_package_and_the_allowed_kinds() {
    let violation = PurityViolation::MissingKind {
        package: "app".to_owned(),
    };

    assert_eq!(
        describe_violation(&violation),
        "app: no [package.metadata.standards] kind (core, adapter, app or tool)"
    );
}

#[test]
fn describes_impure_dependency_with_the_package_and_the_dependency() {
    let violation = PurityViolation::ImpureDependency {
        package: "pricing".to_owned(),
        dependency: "reqwest".to_owned(),
    };

    assert_eq!(
        describe_violation(&violation),
        "pricing: core crate depends on reqwest, which is not a workspace core crate"
    );
}

#[test]
fn ends_without_a_newline_for_missing_kind() {
    let violation = PurityViolation::MissingKind {
        package: "app".to_owned(),
    };

    assert!(!describe_violation(&violation).ends_with('\n'));
}

#[test]
fn ends_without_a_newline_for_impure_dependency() {
    let violation = PurityViolation::ImpureDependency {
        package: "pricing".to_owned(),
        dependency: "reqwest".to_owned(),
    };

    assert!(!describe_violation(&violation).ends_with('\n'));
}

#[test]
fn inserts_a_missing_kind_package_name_without_quoting_it() {
    let violation = PurityViolation::MissingKind {
        package: "odd name \"quoted\"".to_owned(),
    };

    assert_eq!(
        describe_violation(&violation),
        "odd name \"quoted\": no [package.metadata.standards] kind (core, adapter, app or tool)"
    );
}

#[test]
fn inserts_an_impure_dependency_name_without_quoting_it() {
    let violation = PurityViolation::ImpureDependency {
        package: "core crate".to_owned(),
        dependency: "'dep'".to_owned(),
    };

    assert_eq!(
        describe_violation(&violation),
        "core crate: core crate depends on 'dep', which is not a workspace core crate"
    );
}

#[test]
fn describes_missing_kind_when_the_package_name_is_empty() {
    let violation = PurityViolation::MissingKind {
        package: String::new(),
    };

    assert_eq!(
        describe_violation(&violation),
        ": no [package.metadata.standards] kind (core, adapter, app or tool)"
    );
}

#[test]
fn describes_impure_dependency_when_both_names_are_empty() {
    let violation = PurityViolation::ImpureDependency {
        package: String::new(),
        dependency: String::new(),
    };

    assert_eq!(
        describe_violation(&violation),
        ": core crate depends on , which is not a workspace core crate"
    );
}

proptest! {
    #[test]
    fn contains_no_newline_when_the_names_contain_none(
        package in "[^\r\n]{0,16}",
        dependency in "[^\r\n]{0,16}",
    ) {
        let missing = describe_violation(&PurityViolation::MissingKind {
            package: package.clone(),
        });
        prop_assert!(!missing.contains('\n'), "missing kind description contained a newline: {missing:?}");

        let impure = describe_violation(&PurityViolation::ImpureDependency { package, dependency });
        prop_assert!(!impure.contains('\n'), "impure dependency description contained a newline: {impure:?}");
    }

    #[test]
    fn starts_with_the_package_name_followed_by_a_colon_and_a_space(
        package in prop::collection::vec(any::<char>(), 0..16).prop_map(|chars| chars.into_iter().collect::<String>()),
        dependency in prop::collection::vec(any::<char>(), 0..16).prop_map(|chars| chars.into_iter().collect::<String>()),
    ) {
        let prefix = format!("{package}: ");

        let missing = describe_violation(&PurityViolation::MissingKind {
            package: package.clone(),
        });
        prop_assert!(missing.starts_with(&prefix), "{missing:?} does not start with {prefix:?}");

        let impure = describe_violation(&PurityViolation::ImpureDependency { package, dependency });
        prop_assert!(impure.starts_with(&prefix), "{impure:?} does not start with {prefix:?}");
    }
}
