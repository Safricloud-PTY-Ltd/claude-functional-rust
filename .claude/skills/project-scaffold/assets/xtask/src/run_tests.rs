//! Contract tests for `run`.

use proptest::prelude::*;

use super::run;
use crate::core::check_steps;
use crate::shell::FakeRunCommand;
use crate::types::{Captured, CommandLine, Expectation, RunOutput, SpawnFailed, Step};

/// `cargo metadata` output describing a workspace the purity rule has nothing to say about.
const PURE_METADATA: &str = r#"{"packages":[{"name":"shared","metadata":{"standards":{"kind":"core"}},"dependencies":[]}]}"#;

/// What a favourable run of every step prints, in `check_steps` order.
const ALL_OK_REPORT: &str = concat!(
    "ok    fmt\n",
    "ok    clippy\n",
    "ok    test\n",
    "ok    doctest\n",
    "ok    deny\n",
    "ok    machete\n",
    "ok    no-stubs\n",
    "ok    purity\n",
    "check: 8 passed, 0 failed\n",
);

/// Answers one step the way a clean workspace would: the exit code its expectation calls a
/// pass, and for the purity step [`PURE_METADATA`] on standard output.
fn favourable_answer(step: &Step) -> Captured {
    let (exit_code, stdout) = match step.expectation {
        Expectation::ExitZero => (0, ""),
        Expectation::NoMatches => (1, ""),
        Expectation::PureCoreCrates => (0, PURE_METADATA),
    };
    Captured {
        exit_code: Some(exit_code),
        stdout: String::from(stdout),
        stderr: String::new(),
    }
}

#[test]
fn returns_exit_code_two_and_the_usage_lines_when_there_are_no_arguments() {
    let args: Vec<String> = Vec::new();
    // An empty fake answers every command line with a spawn failure, so a check run against it
    // exits 1 with eight FAIL lines; exit code 2 is only reachable without calling the runner.
    let runner = FakeRunCommand::new(Vec::new());

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 2,
            report: String::from("no command given\nusage: cargo xtask check\n"),
        }
    );
}

#[test]
fn returns_exit_code_two_and_names_the_argument_when_the_command_is_unknown() {
    let args = vec![String::from("build")];
    // As above: an empty fake would have produced failures, and exit code 2 rules that out.
    let runner = FakeRunCommand::new(Vec::new());

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 2,
            report: String::from("unknown command: build\nusage: cargo xtask check\n"),
        }
    );
}

#[test]
fn returns_exit_code_two_and_names_the_argument_when_the_command_is_capitalised_check() {
    let args = vec![String::from("Check")];
    let runner = FakeRunCommand::new(Vec::new());

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 2,
            report: String::from("unknown command: Check\nusage: cargo xtask check\n"),
        }
    );
}

#[test]
fn returns_exit_code_two_and_an_empty_name_when_the_command_is_the_empty_string() {
    let args = vec![String::new()];
    let runner = FakeRunCommand::new(Vec::new());

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 2,
            report: String::from("unknown command: \nusage: cargo xtask check\n"),
        }
    );
}

#[test]
fn returns_exit_code_zero_and_an_ok_line_per_step_when_every_step_is_answered_favourably() {
    let responses: Vec<(CommandLine, Result<Captured, SpawnFailed>)> = check_steps()
        .into_iter()
        .map(|step| {
            let captured = favourable_answer(&step);
            (step.command_line, Ok(captured))
        })
        .collect();
    let runner = FakeRunCommand::new(responses);
    let args = vec![String::from("check")];

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 0,
            report: String::from(ALL_OK_REPORT),
        }
    );
}

#[test]
fn returns_exit_code_zero_and_the_same_report_when_check_is_followed_by_further_arguments() {
    let responses: Vec<(CommandLine, Result<Captured, SpawnFailed>)> = check_steps()
        .into_iter()
        .map(|step| {
            let captured = favourable_answer(&step);
            (step.command_line, Ok(captured))
        })
        .collect();
    let runner = FakeRunCommand::new(responses);
    let args = vec![
        String::from("check"),
        String::from("--fast"),
        String::from("nonsense"),
    ];

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 0,
            report: String::from(ALL_OK_REPORT),
        }
    );
}

#[test]
fn judges_every_later_step_when_a_step_in_the_middle_fails() {
    let responses: Vec<(CommandLine, Result<Captured, SpawnFailed>)> = check_steps()
        .into_iter()
        .map(|step| {
            let captured = if step.name == "deny" {
                Captured {
                    exit_code: Some(101),
                    stdout: String::from("error: banned licence\n"),
                    stderr: String::new(),
                }
            } else {
                favourable_answer(&step)
            };
            (step.command_line, Ok(captured))
        })
        .collect();
    let runner = FakeRunCommand::new(responses);
    let args = vec![String::from("check")];

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 1,
            report: String::from(concat!(
                "ok    fmt\n",
                "ok    clippy\n",
                "ok    test\n",
                "ok    doctest\n",
                "FAIL  deny\n",
                "    error: banned licence\n",
                "ok    machete\n",
                "ok    no-stubs\n",
                "ok    purity\n",
                "check: 7 passed, 1 failed\n",
            )),
        }
    );
}

#[test]
fn fails_every_step_with_the_spawn_detail_when_the_fake_answers_nothing() {
    let runner = FakeRunCommand::new(Vec::new());
    let args = vec![String::from("check")];

    assert_eq!(
        run(&args, &runner),
        RunOutput {
            exit_code: 1,
            report: String::from(concat!(
                "FAIL  fmt\n",
                "    could not start cargo: no fake response for this command line\n",
                "FAIL  clippy\n",
                "    could not start cargo: no fake response for this command line\n",
                "FAIL  test\n",
                "    could not start cargo: no fake response for this command line\n",
                "FAIL  doctest\n",
                "    could not start cargo: no fake response for this command line\n",
                "FAIL  deny\n",
                "    could not start cargo: no fake response for this command line\n",
                "FAIL  machete\n",
                "    could not start cargo-machete: no fake response for this command line\n",
                "FAIL  no-stubs\n",
                "    could not start git: no fake response for this command line\n",
                "FAIL  purity\n",
                "    could not start cargo: no fake response for this command line\n",
                "check: 0 passed, 8 failed\n",
            )),
        }
    );
}

proptest! {
    #[test]
    fn returns_exit_code_two_and_the_two_line_report_for_any_command_other_than_check(
        given in any::<String>().prop_filter("check is the one known command", |s| s != "check"),
    ) {
        let args = vec![given.clone()];
        let runner = FakeRunCommand::new(Vec::new());

        prop_assert_eq!(
            run(&args, &runner),
            RunOutput {
                exit_code: 2,
                report: format!("unknown command: {given}\nusage: cargo xtask check\n"),
            }
        );
    }
}
