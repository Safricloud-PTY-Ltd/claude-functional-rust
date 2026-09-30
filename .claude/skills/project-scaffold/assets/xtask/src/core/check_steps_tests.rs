//! Contract tests for `check_steps`.

use std::collections::BTreeSet;

use super::check_steps;
use crate::types::{CommandLine, Expectation, Step};

#[test]
fn returns_eight_steps() {
    assert_eq!(check_steps().len(), 8);
}

#[test]
fn returns_the_eight_gate_names_in_order() {
    let names: Vec<String> = check_steps().into_iter().map(|step| step.name).collect();
    assert_eq!(
        names,
        [
            "fmt", "clippy", "test", "doctest", "deny", "machete", "no-stubs", "purity",
        ]
        .map(String::from)
        .to_vec()
    );
}

#[test]
fn runs_cargo_for_every_step_except_machete_and_no_stubs() {
    let programs: Vec<String> = check_steps()
        .into_iter()
        .map(|step| step.command_line.program)
        .collect();
    assert_eq!(
        programs,
        [
            "cargo",
            "cargo",
            "cargo",
            "cargo",
            "cargo",
            "cargo-machete",
            "git",
            "cargo",
        ]
        .map(String::from)
        .to_vec()
    );
}

#[test]
fn first_step_checks_formatting_across_the_workspace() {
    assert_eq!(
        check_steps()[0],
        Step {
            name: "fmt".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: ["fmt", "--all", "--check"].map(String::from).to_vec(),
            },
            expectation: Expectation::ExitZero,
        }
    );
}

#[test]
fn second_step_runs_clippy_over_all_targets_denying_warnings() {
    assert_eq!(
        check_steps()[1],
        Step {
            name: "clippy".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: [
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ]
                .map(String::from)
                .to_vec(),
            },
            expectation: Expectation::ExitZero,
        }
    );
}

#[test]
fn third_step_runs_the_workspace_tests_with_nextest() {
    assert_eq!(
        check_steps()[2],
        Step {
            name: "test".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: ["nextest", "run", "--workspace", "--no-tests=pass"]
                    .map(String::from)
                    .to_vec(),
            },
            expectation: Expectation::ExitZero,
        }
    );
}

#[test]
fn fourth_step_runs_the_workspace_doctests() {
    assert_eq!(
        check_steps()[3],
        Step {
            name: "doctest".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: ["test", "--workspace", "--doc"].map(String::from).to_vec(),
            },
            expectation: Expectation::ExitZero,
        }
    );
}

#[test]
fn fifth_step_checks_licences_and_advisories_with_deny() {
    assert_eq!(
        check_steps()[4],
        Step {
            name: "deny".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: ["deny", "check"].map(String::from).to_vec(),
            },
            expectation: Expectation::ExitZero,
        }
    );
}

#[test]
fn sixth_step_starts_the_cargo_machete_binary_with_no_arguments() {
    assert_eq!(
        check_steps()[5],
        Step {
            name: "machete".to_owned(),
            command_line: CommandLine {
                program: "cargo-machete".to_owned(),
                args: Vec::new(),
            },
            expectation: Expectation::ExitZero,
        }
    );
}

#[test]
fn seventh_step_greps_rust_files_outside_shared_for_a_stub_body() {
    let pattern = "^[[:space:]]*stub!\\(";
    assert_eq!(pattern.chars().count(), 20);
    assert_eq!(
        check_steps()[6],
        Step {
            name: "no-stubs".to_owned(),
            command_line: CommandLine {
                program: "git".to_owned(),
                args: ["grep", "-n", "-E", pattern, "--", "*.rs", ":!crates/shared",]
                    .map(String::from)
                    .to_vec(),
            },
            expectation: Expectation::NoMatches,
        }
    );
}

#[test]
fn eighth_step_reads_workspace_metadata_to_judge_core_purity() {
    assert_eq!(
        check_steps()[7],
        Step {
            name: "purity".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: ["metadata", "--format-version", "1", "--no-deps"]
                    .map(String::from)
                    .to_vec(),
            },
            expectation: Expectation::PureCoreCrates,
        }
    );
}

#[test]
fn step_names_are_unique() {
    let steps = check_steps();
    let names: BTreeSet<String> = steps.iter().map(|step| step.name.clone()).collect();
    assert_eq!(names.len(), steps.len());
}

#[test]
fn returns_an_equal_value_on_two_calls() {
    assert_eq!(check_steps(), check_steps());
}
