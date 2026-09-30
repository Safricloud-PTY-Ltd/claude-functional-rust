//! Contract tests for `system_run_command`.

use std::path::PathBuf;

use super::SystemRunCommand;
use crate::types::{CommandLine, RunCommand};

#[test]
fn captures_exit_code_zero_and_the_version_on_standard_output_when_the_program_succeeds() {
    let runner = SystemRunCommand::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let captured = runner
        .run(&CommandLine {
            program: "rustc".to_owned(),
            args: vec!["--version".to_owned()],
        })
        .expect("rustc --version should start");

    assert_eq!(captured.exit_code, Some(0));
    assert!(
        captured.stdout.starts_with("rustc "),
        "standard output was {:?}",
        captured.stdout
    );
}

#[test]
fn returns_ok_with_a_non_zero_exit_code_and_standard_error_when_arguments_are_rejected() {
    let runner = SystemRunCommand::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let captured = runner
        .run(&CommandLine {
            program: "rustc".to_owned(),
            args: vec!["--definitely-not-a-flag".to_owned()],
        })
        .expect("a program that starts and then fails is not a spawn failure");

    assert_ne!(captured.exit_code, Some(0));
    assert!(
        !captured.stderr.is_empty(),
        "standard error should carry the complaint"
    );
}

#[test]
fn passes_arguments_to_the_program_without_shell_interpretation() {
    let runner = SystemRunCommand::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let argument = "no such print$(echo hi)";

    let captured = runner
        .run(&CommandLine {
            program: "rustc".to_owned(),
            args: vec!["--print".to_owned(), argument.to_owned()],
        })
        .expect("rustc should start");

    assert!(
        captured.stderr.contains(argument),
        "the argument should reach rustc verbatim; standard error was {:?}",
        captured.stderr
    );
}

#[test]
fn returns_spawn_failed_with_the_program_name_and_a_reason_when_the_program_does_not_exist() {
    let runner = SystemRunCommand::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let program = "xtask-no-such-program-anywhere-on-this-machine";

    let result = runner.run(&CommandLine {
        program: program.to_owned(),
        args: Vec::new(),
    });

    let error = result.expect_err("a program that cannot be found cannot be started");
    assert_eq!(error.program, program);
    assert!(
        !error.reason.is_empty(),
        "the operating system's explanation should be carried through"
    );
}

#[test]
fn returns_spawn_failed_when_the_working_directory_does_not_exist() {
    let missing = std::env::temp_dir().join(format!(
        "xtask-system-run-command-{}-working-directory-does-not-exist",
        std::process::id()
    ));
    let runner = SystemRunCommand::new(missing);

    let result = runner.run(&CommandLine {
        program: "rustc".to_owned(),
        args: vec!["--version".to_owned()],
    });

    let error = result.expect_err("a process cannot start in a directory that is not there");
    assert_eq!(error.program, "rustc");
    assert!(
        !error.reason.is_empty(),
        "the operating system's explanation should be carried through"
    );
}

#[test]
fn starts_the_program_in_the_configured_working_directory() {
    let runner = SystemRunCommand::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let captured = runner
        .run(&CommandLine {
            program: "cargo".to_owned(),
            args: vec![
                "locate-project".to_owned(),
                "--message-format".to_owned(),
                "plain".to_owned(),
            ],
        })
        .expect("cargo should start");

    assert_eq!(captured.exit_code, Some(0));
    assert!(
        captured.stdout.trim_end().ends_with("Cargo.toml"),
        "standard output was {:?}",
        captured.stdout
    );
}

#[test]
fn starts_the_program_outside_any_cargo_project_when_the_working_directory_is_an_empty_directory() {
    let directory = std::env::temp_dir().join(format!(
        "xtask-system-run-command-{}-empty-working-directory",
        std::process::id()
    ));
    let runner = SystemRunCommand::new(directory.clone());
    std::fs::create_dir_all(&directory).expect("the empty working directory should be creatable");

    let result = runner.run(&CommandLine {
        program: "cargo".to_owned(),
        args: vec![
            "locate-project".to_owned(),
            "--message-format".to_owned(),
            "plain".to_owned(),
        ],
    });

    std::fs::remove_dir_all(&directory).expect("the empty working directory should be removable");
    let captured = result.expect("cargo should start");
    assert_ne!(captured.exit_code, Some(0));
}
