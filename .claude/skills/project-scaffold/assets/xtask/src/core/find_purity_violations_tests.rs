//! Contract tests for `find_purity_violations`.

use proptest::prelude::*;

use super::find_purity_violations;
use crate::types::{CrateKind, Dependency, DependencyScope, Package, PurityViolation};

#[test]
fn returns_no_violations_for_an_empty_workspace() {
    assert_eq!(find_purity_violations(&[]), Vec::new());
}

#[test]
fn reports_missing_kind_when_a_package_declares_no_kind() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: None,
        dependencies: Vec::new(),
    }];

    assert_eq!(
        find_purity_violations(&packages),
        vec![PurityViolation::MissingKind {
            package: "a".to_owned(),
        }]
    );
}

#[test]
fn reports_only_missing_kind_for_a_kindless_package_with_impure_dependencies() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: None,
        dependencies: vec![Dependency {
            name: "serde".to_owned(),
            scope: DependencyScope::Runtime,
        }],
    }];

    assert_eq!(
        find_purity_violations(&packages),
        vec![PurityViolation::MissingKind {
            package: "a".to_owned(),
        }]
    );
}

#[test]
fn reports_no_violation_when_a_core_package_depends_on_a_workspace_core_package() {
    let packages = vec![
        Package {
            name: "a".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: vec![Dependency {
                name: "b".to_owned(),
                scope: DependencyScope::Runtime,
            }],
        },
        Package {
            name: "b".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: Vec::new(),
        },
    ];

    assert_eq!(find_purity_violations(&packages), Vec::new());
}

#[test]
fn reports_impure_dependency_when_a_core_package_depends_on_a_non_workspace_package() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: Some(CrateKind::Core),
        dependencies: vec![Dependency {
            name: "serde".to_owned(),
            scope: DependencyScope::Runtime,
        }],
    }];

    assert_eq!(
        find_purity_violations(&packages),
        vec![PurityViolation::ImpureDependency {
            package: "a".to_owned(),
            dependency: "serde".to_owned(),
        }]
    );
}

#[test]
fn reports_impure_dependency_when_a_core_package_depends_on_a_workspace_adapter() {
    let packages = vec![
        Package {
            name: "a".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: vec![Dependency {
                name: "b".to_owned(),
                scope: DependencyScope::Runtime,
            }],
        },
        Package {
            name: "b".to_owned(),
            kind: Some(CrateKind::Adapter),
            dependencies: Vec::new(),
        },
    ];

    assert_eq!(
        find_purity_violations(&packages),
        vec![PurityViolation::ImpureDependency {
            package: "a".to_owned(),
            dependency: "b".to_owned(),
        }]
    );
}

#[test]
fn reports_impure_dependency_when_a_core_package_depends_on_a_workspace_package_without_a_kind() {
    let packages = vec![
        Package {
            name: "a".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: vec![Dependency {
                name: "b".to_owned(),
                scope: DependencyScope::Runtime,
            }],
        },
        Package {
            name: "b".to_owned(),
            kind: None,
            dependencies: Vec::new(),
        },
    ];

    assert_eq!(
        find_purity_violations(&packages),
        vec![
            PurityViolation::ImpureDependency {
                package: "a".to_owned(),
                dependency: "b".to_owned(),
            },
            PurityViolation::MissingKind {
                package: "b".to_owned(),
            },
        ]
    );
}

#[test]
fn reports_no_violation_for_a_core_packages_dev_dependency() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: Some(CrateKind::Core),
        dependencies: vec![Dependency {
            name: "proptest".to_owned(),
            scope: DependencyScope::Dev,
        }],
    }];

    assert_eq!(find_purity_violations(&packages), Vec::new());
}

#[test]
fn reports_no_violation_for_a_core_packages_build_dependency() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: Some(CrateKind::Core),
        dependencies: vec![Dependency {
            name: "cc".to_owned(),
            scope: DependencyScope::Build,
        }],
    }];

    assert_eq!(find_purity_violations(&packages), Vec::new());
}

#[test]
fn reports_no_violation_when_an_adapter_depends_on_a_non_workspace_package() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: Some(CrateKind::Adapter),
        dependencies: vec![Dependency {
            name: "reqwest".to_owned(),
            scope: DependencyScope::Runtime,
        }],
    }];

    assert_eq!(find_purity_violations(&packages), Vec::new());
}

#[test]
fn reports_no_violation_when_an_app_depends_on_a_non_workspace_package() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: Some(CrateKind::App),
        dependencies: vec![Dependency {
            name: "tokio".to_owned(),
            scope: DependencyScope::Runtime,
        }],
    }];

    assert_eq!(find_purity_violations(&packages), Vec::new());
}

#[test]
fn reports_no_violation_when_a_tool_depends_on_a_non_workspace_package() {
    let packages = vec![Package {
        name: "a".to_owned(),
        kind: Some(CrateKind::Tool),
        dependencies: vec![Dependency {
            name: "serde_json".to_owned(),
            scope: DependencyScope::Runtime,
        }],
    }];

    assert_eq!(find_purity_violations(&packages), Vec::new());
}

#[test]
fn reports_one_impure_dependency_per_offending_dependency_in_dependency_order() {
    let packages = vec![
        Package {
            name: "a".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: vec![
                Dependency {
                    name: "serde".to_owned(),
                    scope: DependencyScope::Runtime,
                },
                Dependency {
                    name: "b".to_owned(),
                    scope: DependencyScope::Runtime,
                },
                Dependency {
                    name: "c".to_owned(),
                    scope: DependencyScope::Runtime,
                },
            ],
        },
        Package {
            name: "b".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: Vec::new(),
        },
        Package {
            name: "c".to_owned(),
            kind: Some(CrateKind::App),
            dependencies: Vec::new(),
        },
    ];

    assert_eq!(
        find_purity_violations(&packages),
        vec![
            PurityViolation::ImpureDependency {
                package: "a".to_owned(),
                dependency: "serde".to_owned(),
            },
            PurityViolation::ImpureDependency {
                package: "a".to_owned(),
                dependency: "c".to_owned(),
            },
        ]
    );
}

#[test]
fn reports_violations_grouped_by_package_in_the_order_of_packages() {
    let packages = vec![
        Package {
            name: "a".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: vec![Dependency {
                name: "serde".to_owned(),
                scope: DependencyScope::Runtime,
            }],
        },
        Package {
            name: "b".to_owned(),
            kind: None,
            dependencies: Vec::new(),
        },
        Package {
            name: "c".to_owned(),
            kind: Some(CrateKind::Core),
            dependencies: vec![Dependency {
                name: "serde_json".to_owned(),
                scope: DependencyScope::Runtime,
            }],
        },
    ];

    assert_eq!(
        find_purity_violations(&packages),
        vec![
            PurityViolation::ImpureDependency {
                package: "a".to_owned(),
                dependency: "serde".to_owned(),
            },
            PurityViolation::MissingKind {
                package: "b".to_owned(),
            },
            PurityViolation::ImpureDependency {
                package: "c".to_owned(),
                dependency: "serde_json".to_owned(),
            },
        ]
    );
}

/// Generates workspaces of distinctly named packages, each with a possibly missing kind and
/// distinctly named dependencies in every scope.
fn workspaces() -> impl Strategy<Value = Vec<Package>> {
    prop::sample::subsequence(vec!["a", "b", "c", "d"], 0..=4).prop_flat_map(|names| {
        let count = names.len();
        (
            Just(names),
            prop::collection::vec(
                prop::option::of(prop::sample::select(vec![
                    CrateKind::Core,
                    CrateKind::Adapter,
                    CrateKind::App,
                    CrateKind::Tool,
                ])),
                count,
            ),
            prop::collection::vec(dependency_lists(), count),
        )
            .prop_map(|(names, kinds, dependency_lists)| {
                names
                    .into_iter()
                    .zip(kinds)
                    .zip(dependency_lists)
                    .map(|((name, kind), dependencies)| Package {
                        name: name.to_owned(),
                        kind,
                        dependencies,
                    })
                    .collect()
            })
    })
}

/// Generates one package's dependency list: distinct names drawn from workspace and
/// non-workspace names alike, each in an arbitrary scope.
fn dependency_lists() -> impl Strategy<Value = Vec<Dependency>> {
    prop::sample::subsequence(vec!["a", "b", "c", "d", "serde", "proptest"], 0..=6).prop_flat_map(
        |names| {
            let count = names.len();
            (
                Just(names),
                prop::collection::vec(
                    prop::sample::select(vec![
                        DependencyScope::Runtime,
                        DependencyScope::Dev,
                        DependencyScope::Build,
                    ]),
                    count,
                ),
            )
                .prop_map(|(names, scopes)| {
                    names
                        .into_iter()
                        .zip(scopes)
                        .map(|(name, scope)| Dependency {
                            name: name.to_owned(),
                            scope,
                        })
                        .collect()
                })
        },
    )
}

proptest! {
    #[test]
    fn reports_no_violation_when_every_package_is_an_adapter_an_app_or_a_tool(
        packages in workspaces().prop_map(|packages| {
            packages
                .into_iter()
                .enumerate()
                .map(|(index, package)| Package {
                    kind: Some(
                        [CrateKind::Tool, CrateKind::Adapter, CrateKind::App][index % 3],
                    ),
                    ..package
                })
                .collect::<Vec<_>>()
        }),
    ) {
        prop_assert_eq!(find_purity_violations(&packages), Vec::new());
    }

    #[test]
    fn reports_one_missing_kind_per_package_without_a_kind(packages in workspaces()) {
        let reported = find_purity_violations(&packages)
            .into_iter()
            .filter(|violation| matches!(violation, PurityViolation::MissingKind { .. }))
            .count();

        prop_assert_eq!(
            reported,
            packages.iter().filter(|package| package.kind.is_none()).count()
        );
    }

    #[test]
    fn appending_dev_and_build_dependencies_never_changes_the_violations(
        packages in workspaces(),
        extra in prop::collection::vec(
            (
                prop::sample::select(vec!["a", "b", "c", "d", "serde", "proptest"]),
                prop::sample::select(vec![DependencyScope::Dev, DependencyScope::Build]),
            ),
            0..4,
        ),
    ) {
        let widened: Vec<Package> = packages
            .iter()
            .map(|package| Package {
                name: package.name.clone(),
                kind: package.kind,
                dependencies: package
                    .dependencies
                    .iter()
                    .cloned()
                    .chain(extra.iter().map(|(name, scope)| Dependency {
                        name: (*name).to_owned(),
                        scope: *scope,
                    }))
                    .collect(),
            })
            .collect();

        prop_assert_eq!(find_purity_violations(&packages), find_purity_violations(&widened));
    }

    #[test]
    fn reports_violations_in_package_order_then_dependency_order(packages in workspaces()) {
        let positions: Vec<(usize, usize)> = find_purity_violations(&packages)
            .iter()
            .map(|violation| match violation {
                PurityViolation::MissingKind { package } => (
                    packages.iter().position(|p| &p.name == package).unwrap(),
                    0,
                ),
                PurityViolation::ImpureDependency {
                    package,
                    dependency,
                } => {
                    let index = packages.iter().position(|p| &p.name == package).unwrap();
                    let dependency_index = packages[index]
                        .dependencies
                        .iter()
                        .position(|d| &d.name == dependency)
                        .unwrap();
                    (index, dependency_index)
                }
            })
            .collect();

        prop_assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
