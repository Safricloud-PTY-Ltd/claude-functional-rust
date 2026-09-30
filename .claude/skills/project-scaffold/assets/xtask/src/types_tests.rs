//! Contract tests for `types`.

use std::error::Error;

use super::{InvalidMetadata, SpawnFailed, UnknownCommand};

#[test]
fn unknown_command_displays_the_given_argument() {
    let error = UnknownCommand {
        given: Some("chek".to_owned()),
    };

    assert_eq!(error.to_string(), "unknown command: chek");
}

#[test]
fn unknown_command_displays_no_command_given_when_given_is_none() {
    let error = UnknownCommand { given: None };

    assert_eq!(error.to_string(), "no command given");
}

#[test]
fn unknown_command_displays_an_empty_argument_when_given_is_the_empty_string() {
    let error = UnknownCommand {
        given: Some(String::new()),
    };

    assert_eq!(error.to_string(), "unknown command: ");
}

#[test]
fn unknown_command_display_does_not_end_in_a_newline() {
    let given = UnknownCommand {
        given: Some("chek".to_owned()),
    }
    .to_string();
    let missing = UnknownCommand { given: None }.to_string();

    assert!(!given.ends_with('\n'));
    assert!(!missing.ends_with('\n'));
}

#[test]
fn spawn_failed_displays_the_program_and_the_reason() {
    let error = SpawnFailed {
        program: "cargo".to_owned(),
        reason: "No such file or directory (os error 2)".to_owned(),
    };

    assert_eq!(
        error.to_string(),
        "could not start cargo: No such file or directory (os error 2)"
    );
}

#[test]
fn spawn_failed_display_does_not_end_in_a_newline() {
    let error = SpawnFailed {
        program: "cargo".to_owned(),
        reason: "permission denied".to_owned(),
    };

    assert!(!error.to_string().ends_with('\n'));
}

#[test]
fn invalid_metadata_displays_the_reason() {
    let error = InvalidMetadata {
        reason: "expected value at line 1 column 1".to_owned(),
    };

    assert_eq!(
        error.to_string(),
        "invalid cargo metadata: expected value at line 1 column 1"
    );
}

#[test]
fn invalid_metadata_display_does_not_end_in_a_newline() {
    let error = InvalidMetadata {
        reason: "trailing characters".to_owned(),
    };

    assert!(!error.to_string().ends_with('\n'));
}

#[test]
fn unknown_command_is_usable_as_a_dyn_error() {
    let error = UnknownCommand {
        given: Some("chek".to_owned()),
    };
    let as_error: &dyn Error = &error;

    assert!(as_error.source().is_none());
}

#[test]
fn spawn_failed_is_usable_as_a_dyn_error() {
    let error = SpawnFailed {
        program: "cargo".to_owned(),
        reason: "permission denied".to_owned(),
    };
    let as_error: &dyn Error = &error;

    assert!(as_error.source().is_none());
}

#[test]
fn invalid_metadata_is_usable_as_a_dyn_error() {
    let error = InvalidMetadata {
        reason: "trailing characters".to_owned(),
    };
    let as_error: &dyn Error = &error;

    assert!(as_error.source().is_none());
}
