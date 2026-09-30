//! Contract tests for `parse_metadata`.

use proptest::prelude::*;

use super::parse_metadata;
use crate::types::{CrateKind, Dependency, DependencyScope, Package};

/// One package element carrying every field `cargo metadata --format-version 1 --no-deps`
/// actually emits, so that "every other field is ignored, whatever its type" is exercised
/// against strings, nulls, numbers, arrays, objects and booleans.
const REALISTIC: &str = r#"
{
  "packages": [
    {
      "id": "path+file:///repo/crates/domain-types#0.0.0",
      "name": "domain-types",
      "version": "0.0.0",
      "license": null,
      "license_file": null,
      "description": "the shared vocabulary",
      "source": null,
      "dependencies": [
        {
          "name": "shared",
          "source": null,
          "req": "*",
          "kind": null,
          "rename": null,
          "optional": false,
          "uses_default_features": true,
          "features": [],
          "target": null,
          "registry": null,
          "path": "/repo/crates/shared"
        },
        {
          "name": "proptest",
          "source": "registry+https://github.com/rust-lang/crates.io-index",
          "req": "^1",
          "kind": "dev",
          "rename": null,
          "optional": false,
          "uses_default_features": true,
          "features": [],
          "target": null,
          "registry": null
        }
      ],
      "targets": [
        {
          "kind": ["lib"],
          "crate_types": ["lib"],
          "name": "domain-types",
          "src_path": "/repo/crates/domain-types/src/lib.rs",
          "edition": "2024",
          "doc": true,
          "doctest": true,
          "test": true
        }
      ],
      "features": {},
      "manifest_path": "/repo/crates/domain-types/Cargo.toml",
      "metadata": { "standards": { "kind": "core" } },
      "publish": [],
      "authors": [],
      "categories": [],
      "keywords": [],
      "readme": null,
      "repository": null,
      "homepage": null,
      "documentation": null,
      "edition": "2024",
      "links": null,
      "default_run": null,
      "rust_version": "1.98"
    }
  ],
  "workspace_members": ["path+file:///repo/crates/domain-types#0.0.0"],
  "workspace_default_members": ["path+file:///repo/crates/domain-types#0.0.0"],
  "resolve": null,
  "target_directory": "/repo/target",
  "version": 1,
  "workspace_root": "/repo",
  "metadata": null
}
"#;

/// One package per recognised kind string, in a fixed order.
const EVERY_KIND: &str = r#"
{
  "packages": [
    { "name": "a", "metadata": { "standards": { "kind": "core" } }, "dependencies": [] },
    { "name": "b", "metadata": { "standards": { "kind": "adapter" } }, "dependencies": [] },
    { "name": "c", "metadata": { "standards": { "kind": "app" } }, "dependencies": [] },
    { "name": "d", "metadata": { "standards": { "kind": "tool" } }, "dependencies": [] }
  ]
}
"#;

/// One package whose four dependencies cover every scope spelling, in a fixed order.
const EVERY_SCOPE: &str = r#"
{
  "packages": [
    {
      "name": "app",
      "dependencies": [
        { "name": "absent-kind" },
        { "name": "null-kind", "kind": null },
        { "name": "dev-kind", "kind": "dev" },
        { "name": "build-kind", "kind": "build" }
      ]
    }
  ]
}
"#;

#[test]
fn returns_an_empty_vector_when_the_packages_array_is_empty() {
    assert_eq!(parse_metadata(r#"{ "packages": [] }"#), Ok(vec![]));
}

#[test]
fn reads_name_kind_and_dependencies_and_ignores_every_other_field() {
    assert_eq!(
        parse_metadata(REALISTIC),
        Ok(vec![Package {
            name: "domain-types".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: vec![
                Dependency {
                    name: "shared".to_owned(),
                    scope: DependencyScope::Runtime
                },
                Dependency {
                    name: "proptest".to_owned(),
                    scope: DependencyScope::Dev
                },
            ],
        }])
    );
}

#[test]
fn maps_core_adapter_app_and_tool_to_their_crate_kinds_in_order() {
    let kinds = parse_metadata(EVERY_KIND)
        .unwrap()
        .into_iter()
        .map(|package| (package.name, package.kind))
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            ("a".to_owned(), Some(CrateKind::Core)),
            ("b".to_owned(), Some(CrateKind::Adapter)),
            ("c".to_owned(), Some(CrateKind::App)),
            ("d".to_owned(), Some(CrateKind::Tool)),
        ]
    );
}

#[test]
fn returns_no_kind_when_metadata_is_null() {
    let json = r#"{ "packages": [{ "name": "a", "metadata": null, "dependencies": [] }] }"#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_no_kind_when_metadata_is_absent() {
    let json = r#"{ "packages": [{ "name": "a", "dependencies": [] }] }"#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_no_kind_when_metadata_has_no_standards_table() {
    let json =
        r#"{ "packages": [{ "name": "a", "metadata": { "docs": {} }, "dependencies": [] }] }"#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_no_kind_when_the_standards_table_has_no_kind() {
    let json = r#"
    { "packages": [{ "name": "a", "metadata": { "standards": {} }, "dependencies": [] }] }
    "#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_no_kind_when_kind_is_an_unrecognised_string() {
    let json = r#"
    {
      "packages": [
        { "name": "a", "metadata": { "standards": { "kind": "core " } }, "dependencies": [] }
      ]
    }
    "#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_no_kind_when_metadata_is_a_number() {
    let json = r#"{ "packages": [{ "name": "a", "metadata": 7, "dependencies": [] }] }"#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_no_kind_when_the_standards_table_is_a_number() {
    let json = r#"
    { "packages": [{ "name": "a", "metadata": { "standards": 7 }, "dependencies": [] }] }
    "#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_no_kind_when_the_kind_value_is_a_number() {
    let json = r#"
    {
      "packages": [
        { "name": "a", "metadata": { "standards": { "kind": 7 } }, "dependencies": [] }
      ]
    }
    "#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn reads_absent_null_dev_and_build_dependency_kinds_as_their_scopes_in_order() {
    assert_eq!(
        parse_metadata(EVERY_SCOPE),
        Ok(vec![Package {
            name: "app".to_owned(),
            kind: None,
            dependencies: vec![
                Dependency {
                    name: "absent-kind".to_owned(),
                    scope: DependencyScope::Runtime
                },
                Dependency {
                    name: "null-kind".to_owned(),
                    scope: DependencyScope::Runtime
                },
                Dependency {
                    name: "dev-kind".to_owned(),
                    scope: DependencyScope::Dev
                },
                Dependency {
                    name: "build-kind".to_owned(),
                    scope: DependencyScope::Build
                },
            ],
        }])
    );
}

#[test]
fn returns_no_dependencies_when_the_dependencies_array_is_empty() {
    let json = r#"{ "packages": [{ "name": "a", "dependencies": [] }] }"#;
    assert_eq!(
        parse_metadata(json),
        Ok(vec![Package {
            name: "a".to_owned(),
            kind: None,
            dependencies: vec![]
        }])
    );
}

#[test]
fn returns_invalid_metadata_when_the_json_is_not_valid_json() {
    let error = parse_metadata("{ \"packages\": [").unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for malformed JSON"
    );
}

#[test]
fn returns_invalid_metadata_when_the_json_is_empty() {
    let error = parse_metadata("").unwrap_err();
    assert!(!error.reason.is_empty(), "reason was empty for empty input");
}

#[test]
fn returns_invalid_metadata_when_there_is_no_top_level_packages_key() {
    let error = parse_metadata(r#"{ "version": 1 }"#).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for missing packages"
    );
}

#[test]
fn returns_invalid_metadata_when_packages_is_not_an_array() {
    let error = parse_metadata(r#"{ "packages": { "name": "a" } }"#).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a non-array packages"
    );
}

#[test]
fn returns_invalid_metadata_when_a_package_has_no_name() {
    let error = parse_metadata(r#"{ "packages": [{ "dependencies": [] }] }"#).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a nameless package"
    );
}

#[test]
fn returns_invalid_metadata_when_a_package_name_is_not_a_string() {
    let json = r#"{ "packages": [{ "name": 7, "dependencies": [] }] }"#;
    let error = parse_metadata(json).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a non-string package name"
    );
}

#[test]
fn returns_invalid_metadata_when_a_package_has_no_dependencies_array() {
    let error = parse_metadata(r#"{ "packages": [{ "name": "a" }] }"#).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for missing dependencies"
    );
}

#[test]
fn returns_invalid_metadata_when_dependencies_is_not_an_array() {
    let json = r#"{ "packages": [{ "name": "a", "dependencies": {} }] }"#;
    let error = parse_metadata(json).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a non-array dependencies"
    );
}

#[test]
fn returns_invalid_metadata_when_a_dependency_has_no_name() {
    let json = r#"{ "packages": [{ "name": "a", "dependencies": [{ "kind": "dev" }] }] }"#;
    let error = parse_metadata(json).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a nameless dependency"
    );
}

#[test]
fn returns_invalid_metadata_when_a_dependency_name_is_not_a_string() {
    let json = r#"{ "packages": [{ "name": "a", "dependencies": [{ "name": null }] }] }"#;
    let error = parse_metadata(json).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a non-string dependency name"
    );
}

#[test]
fn returns_invalid_metadata_when_a_dependency_kind_is_an_unrecognised_string() {
    let json = r#"
    { "packages": [{ "name": "a", "dependencies": [{ "name": "b", "kind": "runtime" }] }] }
    "#;
    let error = parse_metadata(json).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for an unrecognised dependency kind"
    );
}

#[test]
fn returns_invalid_metadata_when_a_dependency_kind_is_a_number() {
    let json = r#"{ "packages": [{ "name": "a", "dependencies": [{ "name": "b", "kind": 7 }] }] }"#;
    let error = parse_metadata(json).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a numeric dependency kind"
    );
}

#[test]
fn returns_invalid_metadata_when_a_dependency_kind_is_a_table() {
    let json = r#"
    { "packages": [{ "name": "a", "dependencies": [{ "name": "b", "kind": { "dev": true } }] }] }
    "#;
    let error = parse_metadata(json).unwrap_err();
    assert!(
        !error.reason.is_empty(),
        "reason was empty for a table dependency kind"
    );
}

proptest! {
    /// The contract's `# Remarks`: pure, and never panics whatever the input. Reaching the
    /// assertion at all is the property; a panic, the stub's included, fails the case.
    #[test]
    fn returns_a_result_without_panicking_for_arbitrary_input(json in ".*") {
        let outcome = parse_metadata(&json);
        prop_assert!(outcome.is_ok() || outcome.is_err());
    }

    /// The contract's `# Errors`: `reason` is never empty.
    #[test]
    fn gives_a_non_empty_reason_for_every_rejected_input(json in ".*") {
        if let Err(error) = parse_metadata(&json) {
            prop_assert!(!error.reason.is_empty());
        }
    }

    /// The contract's `# Returns`: one package per element in the same order, each with its
    /// dependencies in the order given.
    #[test]
    fn preserves_the_order_of_packages_and_of_dependencies(
        names in proptest::collection::vec("[a-z][a-z0-9_]{0,7}", 0..4),
        dependency_names in proptest::collection::vec("[a-z][a-z0-9_]{0,7}", 0..4),
    ) {
        let dependencies = dependency_names
            .iter()
            .map(|name| format!("{{ \"name\": \"{name}\" }}"))
            .collect::<Vec<_>>()
            .join(",");
        let packages = names
            .iter()
            .map(|name| format!("{{ \"name\": \"{name}\", \"dependencies\": [{dependencies}] }}"))
            .collect::<Vec<_>>()
            .join(",");
        let parsed = parse_metadata(&format!("{{ \"packages\": [{packages}] }}")).unwrap();

        let parsed_names = parsed.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        prop_assert_eq!(parsed_names, names);
        for package in &parsed {
            let listed = package.dependencies.iter().map(|d| d.name.clone()).collect::<Vec<_>>();
            prop_assert_eq!(listed, dependency_names.clone());
        }
    }
}
