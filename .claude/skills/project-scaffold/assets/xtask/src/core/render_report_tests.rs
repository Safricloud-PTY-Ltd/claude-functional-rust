//! Contract tests for `render_report`.

use proptest::prelude::*;

use super::render_report;
use crate::types::{StepOutcome, Verdict};

#[test]
fn renders_only_the_summary_line_when_there_are_no_outcomes() {
    assert_eq!(render_report(&[]), "check: 0 passed, 0 failed\n");
}

#[test]
fn renders_a_pass_as_ok_then_four_spaces_then_the_name() {
    let outcomes = vec![StepOutcome {
        name: "fmt".to_owned(),
        verdict: Verdict::Passed,
    }];

    assert_eq!(
        render_report(&outcomes),
        "ok    fmt\ncheck: 1 passed, 0 failed\n"
    );
}

#[test]
fn renders_a_failure_as_fail_then_two_spaces_then_the_name() {
    let outcomes = vec![StepOutcome {
        name: "clippy".to_owned(),
        verdict: Verdict::Failed {
            detail: "one bad line".to_owned(),
        },
    }];

    assert_eq!(
        render_report(&outcomes),
        "FAIL  clippy\n    one bad line\ncheck: 0 passed, 1 failed\n"
    );
}

#[test]
fn adds_no_detail_lines_for_a_failure_whose_detail_is_empty() {
    let outcomes = vec![StepOutcome {
        name: "no-stubs".to_owned(),
        verdict: Verdict::Failed {
            detail: String::new(),
        },
    }];

    assert_eq!(
        render_report(&outcomes),
        "FAIL  no-stubs\ncheck: 0 passed, 1 failed\n"
    );
}

#[test]
fn indents_every_line_of_a_multi_line_detail_by_four_spaces() {
    let outcomes = vec![StepOutcome {
        name: "clippy".to_owned(),
        verdict: Verdict::Failed {
            detail: "first\nsecond\nthird".to_owned(),
        },
    }];

    assert_eq!(
        render_report(&outcomes),
        "FAIL  clippy\n    first\n    second\n    third\ncheck: 0 passed, 1 failed\n"
    );
}

#[test]
fn keeps_an_empty_line_inside_a_detail_as_exactly_four_spaces() {
    let outcomes = vec![StepOutcome {
        name: "clippy".to_owned(),
        verdict: Verdict::Failed {
            detail: "first\n\nthird".to_owned(),
        },
    }];

    assert_eq!(
        render_report(&outcomes),
        "FAIL  clippy\n    first\n    \n    third\ncheck: 0 passed, 1 failed\n"
    );
}

#[test]
fn drops_the_carriage_return_immediately_before_a_newline_in_a_detail() {
    let outcomes = vec![StepOutcome {
        name: "deny".to_owned(),
        verdict: Verdict::Failed {
            detail: "first\r\nsecond\r\n\r\nfourth".to_owned(),
        },
    }];

    assert_eq!(
        render_report(&outcomes),
        "FAIL  deny\n    first\n    second\n    \n    fourth\ncheck: 0 passed, 1 failed\n"
    );
}

#[test]
fn keeps_a_carriage_return_that_is_not_immediately_before_a_newline() {
    let outcomes = vec![StepOutcome {
        name: "deny".to_owned(),
        verdict: Verdict::Failed {
            detail: "a\rb\r\r\nc".to_owned(),
        },
    }];

    assert_eq!(
        render_report(&outcomes),
        "FAIL  deny\n    a\rb\r\n    c\ncheck: 0 passed, 1 failed\n"
    );
}

#[test]
fn adds_a_four_space_line_for_a_detail_that_ends_with_a_newline() {
    let outcomes = vec![StepOutcome {
        name: "machete".to_owned(),
        verdict: Verdict::Failed {
            detail: "only\n".to_owned(),
        },
    }];

    assert_eq!(
        render_report(&outcomes),
        "FAIL  machete\n    only\n    \ncheck: 0 passed, 1 failed\n"
    );
}

#[test]
fn keeps_the_outcomes_in_the_order_they_were_given() {
    let outcomes = vec![
        StepOutcome {
            name: "fmt".to_owned(),
            verdict: Verdict::Passed,
        },
        StepOutcome {
            name: "clippy".to_owned(),
            verdict: Verdict::Failed {
                detail: "bad".to_owned(),
            },
        },
        StepOutcome {
            name: "deny".to_owned(),
            verdict: Verdict::Passed,
        },
    ];

    assert_eq!(
        render_report(&outcomes),
        concat!(
            "ok    fmt\n",
            "FAIL  clippy\n",
            "    bad\n",
            "ok    deny\n",
            "check: 2 passed, 1 failed\n"
        )
    );
}

#[test]
fn counts_every_pass_and_every_failure_in_the_last_line() {
    let outcomes = vec![
        StepOutcome {
            name: "a".to_owned(),
            verdict: Verdict::Failed {
                detail: String::new(),
            },
        },
        StepOutcome {
            name: "b".to_owned(),
            verdict: Verdict::Passed,
        },
        StepOutcome {
            name: "c".to_owned(),
            verdict: Verdict::Failed {
                detail: String::new(),
            },
        },
        StepOutcome {
            name: "d".to_owned(),
            verdict: Verdict::Failed {
                detail: String::new(),
            },
        },
    ];

    assert_eq!(
        render_report(&outcomes),
        concat!(
            "FAIL  a\n",
            "ok    b\n",
            "FAIL  c\n",
            "FAIL  d\n",
            "check: 1 passed, 3 failed\n"
        )
    );
}

proptest! {
    #[test]
    fn output_always_ends_with_a_newline(
        outcomes in proptest::collection::vec(
            (
                "[-a-z \\n\\r]{0,8}",
                prop_oneof![
                    Just(Verdict::Passed),
                    "[-a-z \\n\\r]{0,12}".prop_map(|detail| Verdict::Failed { detail }),
                ],
            )
                .prop_map(|(name, verdict)| StepOutcome { name, verdict }),
            0..6_usize,
        )
    ) {
        let report = render_report(&outcomes);

        prop_assert!(report.ends_with('\n'), "report was {report:?}");
    }

    #[test]
    fn passed_and_failed_counts_sum_to_the_number_of_outcomes(
        outcomes in proptest::collection::vec(
            (
                "[-a-z \\n\\r]{0,8}",
                prop_oneof![
                    Just(Verdict::Passed),
                    "[-a-z \\n\\r]{0,12}".prop_map(|detail| Verdict::Failed { detail }),
                ],
            )
                .prop_map(|(name, verdict)| StepOutcome { name, verdict }),
            0..6_usize,
        )
    ) {
        let report = render_report(&outcomes);
        let last = report.lines().last().unwrap();
        let counts = last.strip_prefix("check: ").unwrap();
        let (passed, rest) = counts.split_once(" passed, ").unwrap();
        let failed = rest.strip_suffix(" failed").unwrap();

        prop_assert_eq!(
            passed.parse::<usize>().unwrap() + failed.parse::<usize>().unwrap(),
            outcomes.len()
        );
    }

    #[test]
    fn has_one_line_per_outcome_plus_the_summary_when_no_detail_is_shown(
        outcomes in proptest::collection::vec(
            (
                "[-a-z ]{0,8}",
                prop_oneof![
                    Just(Verdict::Passed),
                    Just(Verdict::Failed { detail: String::new() }),
                ],
            )
                .prop_map(|(name, verdict)| StepOutcome { name, verdict }),
            0..6_usize,
        )
    ) {
        let report = render_report(&outcomes);

        prop_assert_eq!(report.matches('\n').count(), outcomes.len() + 1);
    }
}
