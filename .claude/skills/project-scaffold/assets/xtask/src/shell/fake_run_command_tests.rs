//! Contract tests for `fake_run_command`.

use proptest::prelude::*;

use super::FakeRunCommand;
use crate::types::{Captured, CommandLine, RunCommand, SpawnFailed};

#[test]
fn returns_the_listed_ok_when_the_command_line_is_listed() {
    let command_line = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("fmt"), String::from("--check")],
    };
    let captured = Captured {
        exit_code: Some(0),
        stdout: String::from("all formatted\n"),
        stderr: String::new(),
    };
    let fake = FakeRunCommand::new(vec![(command_line.clone(), Ok(captured.clone()))]);

    assert_eq!(fake.run(&command_line), Ok(captured));
}

#[test]
fn returns_the_listed_err_when_the_command_line_is_listed() {
    let command_line = CommandLine {
        program: String::from("nosuchtool"),
        args: Vec::new(),
    };
    let spawn_failed = SpawnFailed {
        program: String::from("nosuchtool"),
        reason: String::from("program not found"),
    };
    let fake = FakeRunCommand::new(vec![(command_line.clone(), Err(spawn_failed.clone()))]);

    assert_eq!(fake.run(&command_line), Err(spawn_failed));
}

#[test]
fn returns_spawn_failed_with_the_program_and_the_documented_reason_when_the_command_line_is_unlisted()
 {
    let listed = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("fmt")],
    };
    let captured = Captured {
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
    };
    let fake = FakeRunCommand::new(vec![(listed, Ok(captured))]);
    let asked = CommandLine {
        program: String::from("git"),
        args: vec![String::from("status")],
    };

    assert_eq!(
        fake.run(&asked),
        Err(SpawnFailed {
            program: String::from("git"),
            reason: String::from("no fake response for this command line"),
        })
    );
}

#[test]
fn returns_spawn_failed_when_the_program_is_listed_with_different_arguments() {
    let listed = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("fmt")],
    };
    let captured = Captured {
        exit_code: Some(0),
        stdout: String::from("listed"),
        stderr: String::new(),
    };
    let fake = FakeRunCommand::new(vec![(listed, Ok(captured))]);
    let asked = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("fmt"), String::from("--check")],
    };

    assert_eq!(
        fake.run(&asked),
        Err(SpawnFailed {
            program: String::from("cargo"),
            reason: String::from("no fake response for this command line"),
        })
    );
}

#[test]
fn returns_spawn_failed_when_the_listed_arguments_are_in_a_different_order() {
    let listed = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("clippy"), String::from("--all-targets")],
    };
    let captured = Captured {
        exit_code: Some(0),
        stdout: String::from("listed"),
        stderr: String::new(),
    };
    let fake = FakeRunCommand::new(vec![(listed, Ok(captured))]);
    let asked = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("--all-targets"), String::from("clippy")],
    };

    assert_eq!(
        fake.run(&asked),
        Err(SpawnFailed {
            program: String::from("cargo"),
            reason: String::from("no fake response for this command line"),
        })
    );
}

#[test]
fn returns_the_first_result_when_the_command_line_is_listed_twice() {
    let command_line = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("test")],
    };
    let first = Captured {
        exit_code: Some(0),
        stdout: String::from("first"),
        stderr: String::new(),
    };
    let second = Captured {
        exit_code: Some(1),
        stdout: String::from("second"),
        stderr: String::from("boom"),
    };
    let fake = FakeRunCommand::new(vec![
        (command_line.clone(), Ok(first.clone())),
        (command_line.clone(), Ok(second)),
    ]);

    assert_eq!(fake.run(&command_line), Ok(first));
}

#[test]
fn returns_spawn_failed_for_every_command_line_when_the_table_is_empty() {
    let fake = FakeRunCommand::new(Vec::new());
    let asked = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("check")],
    };

    assert_eq!(
        fake.run(&asked),
        Err(SpawnFailed {
            program: String::from("cargo"),
            reason: String::from("no fake response for this command line"),
        })
    );
}

#[test]
fn returns_equal_answers_when_the_same_command_line_is_run_twice() {
    let listed = CommandLine {
        program: String::from("cargo"),
        args: vec![String::from("build")],
    };
    let captured = Captured {
        exit_code: Some(0),
        stdout: String::from("built"),
        stderr: String::new(),
    };
    let fake = FakeRunCommand::new(vec![(listed.clone(), Ok(captured))]);
    let unlisted = CommandLine {
        program: String::from("git"),
        args: Vec::new(),
    };

    assert_eq!(fake.run(&listed), fake.run(&listed));
    assert_eq!(fake.run(&unlisted), fake.run(&unlisted));
}

proptest! {
    #[test]
    fn returns_the_first_paired_result_for_every_listed_command_line_of_any_table(
        rows in prop::collection::vec(
            (
                "(cargo|git|rustc)",
                prop::collection::vec("(check|fmt|--all-targets)", 0..3),
                prop::option::of("[a-z]{1,4}"),
            ),
            0..6,
        ),
    ) {
        let responses: Vec<(CommandLine, Result<Captured, SpawnFailed>)> = rows
            .iter()
            .map(|(program, args, ok)| {
                (
                    CommandLine { program: program.clone(), args: args.clone() },
                    match ok {
                        Some(stdout) => Ok(Captured {
                            exit_code: Some(0),
                            stdout: stdout.clone(),
                            stderr: String::new(),
                        }),
                        None => Err(SpawnFailed {
                            program: program.clone(),
                            reason: String::from("listed failure"),
                        }),
                    },
                )
            })
            .collect();
        let fake = FakeRunCommand::new(responses.clone());

        for (command_line, _) in &responses {
            let first_paired = responses
                .iter()
                .find(|(listed, _)| listed == command_line)
                .map(|(_, result)| result.clone());

            prop_assert_eq!(Some(fake.run(command_line)), first_paired);
        }
    }
}
