//! Contract tests for `parse_command`.

use proptest::prelude::*;

use super::parse_command;
use crate::types::{Command, UnknownCommand};

#[test]
fn returns_check_when_the_only_argument_is_check() {
    let args = vec![String::from("check")];

    assert_eq!(parse_command(&args), Ok(Command::Check));
}

#[test]
fn returns_unknown_command_with_no_argument_when_args_is_empty() {
    let args: Vec<String> = Vec::new();

    assert_eq!(parse_command(&args), Err(UnknownCommand { given: None }));
}

#[test]
fn returns_unknown_command_with_the_argument_when_the_first_argument_is_another_word() {
    let args = vec![String::from("build")];

    assert_eq!(
        parse_command(&args),
        Err(UnknownCommand {
            given: Some(String::from("build")),
        })
    );
}

#[test]
fn returns_unknown_command_with_the_argument_when_the_first_argument_is_capitalised_check() {
    let args = vec![String::from("Check")];

    assert_eq!(
        parse_command(&args),
        Err(UnknownCommand {
            given: Some(String::from("Check")),
        })
    );
}

#[test]
fn returns_unknown_command_with_the_argument_when_the_first_argument_has_a_leading_space() {
    let args = vec![String::from(" check")];

    assert_eq!(
        parse_command(&args),
        Err(UnknownCommand {
            given: Some(String::from(" check")),
        })
    );
}

#[test]
fn returns_unknown_command_with_the_argument_when_the_first_argument_is_the_empty_string() {
    let args = vec![String::new()];

    assert_eq!(
        parse_command(&args),
        Err(UnknownCommand {
            given: Some(String::new()),
        })
    );
}

#[test]
fn returns_check_when_check_is_followed_by_further_arguments() {
    let args = vec![
        String::from("check"),
        String::from("--fast"),
        String::from("check"),
    ];

    assert_eq!(parse_command(&args), Ok(Command::Check));
}

#[test]
fn returns_unknown_command_with_the_first_argument_when_it_is_followed_by_check() {
    let args = vec![String::from("build"), String::from("check")];

    assert_eq!(
        parse_command(&args),
        Err(UnknownCommand {
            given: Some(String::from("build")),
        })
    );
}

proptest! {
    #[test]
    fn returns_unknown_command_with_the_first_argument_for_any_first_argument_other_than_check(
        first in any::<String>().prop_filter("the check command is the one known command", |s| s != "check"),
    ) {
        let args = vec![first.clone()];

        prop_assert_eq!(
            parse_command(&args),
            Err(UnknownCommand { given: Some(first) })
        );
    }

    #[test]
    fn returns_the_same_result_however_many_arguments_follow_the_first(
        first in any::<String>(),
        rest in prop::collection::vec(any::<String>(), 0..4),
    ) {
        let alone = vec![first.clone()];
        let mut with_rest = alone.clone();
        with_rest.extend(rest);

        prop_assert_eq!(parse_command(&alone), parse_command(&with_rest));
    }
}
