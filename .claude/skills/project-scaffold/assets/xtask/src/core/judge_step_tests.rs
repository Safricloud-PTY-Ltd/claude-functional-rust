//! Contract tests for `judge_step`.

use proptest::prelude::*;

use super::judge_step;
use crate::types::{Captured, CommandLine, Expectation, SpawnFailed, Step, StepOutcome, Verdict};

/// A pure workspace: two core crates, the second depending on the first.
const PURE_METADATA: &str = r#"{
  "packages": [
    {
      "name": "shared",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": []
    },
    {
      "name": "domain-types",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": [{ "name": "shared", "kind": null }]
    }
  ]
}"#;

/// A workspace with exactly two violations: an impure core crate, then a crate with no kind.
const TWO_VIOLATIONS_METADATA: &str = r#"{
  "packages": [
    {
      "name": "shared",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": []
    },
    {
      "name": "pricing",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": [{ "name": "reqwest" }]
    },
    {
      "name": "app",
      "dependencies": []
    }
  ]
}"#;

#[test]
fn fails_with_the_spawn_error_text_under_exit_zero() {
    let step = Step {
        name: "fmt".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["fmt".to_owned()],
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Err(SpawnFailed {
                program: "cargo".to_owned(),
                reason: "program not found".to_owned(),
            })
        ),
        StepOutcome {
            name: "fmt".to_owned(),
            verdict: Verdict::Failed {
                detail: "could not start cargo: program not found".to_owned(),
            },
        }
    );
}

#[test]
fn fails_with_the_spawn_error_text_under_no_matches() {
    let step = Step {
        name: "no-stubs".to_owned(),
        command_line: CommandLine {
            program: "rg".to_owned(),
            args: vec!["stub".to_owned()],
        },
        expectation: Expectation::NoMatches,
    };

    assert_eq!(
        judge_step(
            &step,
            Err(SpawnFailed {
                program: "rg".to_owned(),
                reason: "program not found".to_owned(),
            })
        ),
        StepOutcome {
            name: "no-stubs".to_owned(),
            verdict: Verdict::Failed {
                detail: "could not start rg: program not found".to_owned(),
            },
        }
    );
}

#[test]
fn fails_with_the_spawn_error_text_under_pure_core_crates() {
    let step = Step {
        name: "purity".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["metadata".to_owned()],
        },
        expectation: Expectation::PureCoreCrates,
    };

    assert_eq!(
        judge_step(
            &step,
            Err(SpawnFailed {
                program: "cargo".to_owned(),
                reason: "permission denied".to_owned(),
            })
        ),
        StepOutcome {
            name: "purity".to_owned(),
            verdict: Verdict::Failed {
                detail: "could not start cargo: permission denied".to_owned(),
            },
        }
    );
}

#[test]
fn passes_under_exit_zero_when_the_exit_code_is_zero() {
    let step = Step {
        name: "fmt".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["fmt".to_owned()],
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(0),
                stdout: "everything is fine\n".to_owned(),
                stderr: "a warning\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "fmt".to_owned(),
            verdict: Verdict::Passed,
        }
    );
}

#[test]
fn fails_with_the_output_under_exit_zero_when_the_exit_code_is_one() {
    let step = Step {
        name: "clippy".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["clippy".to_owned()],
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(1),
                stdout: "out line\n".to_owned(),
                stderr: "err line\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "clippy".to_owned(),
            verdict: Verdict::Failed {
                detail: "out line\nerr line".to_owned(),
            },
        }
    );
}

#[test]
fn fails_with_the_output_under_exit_zero_when_there_is_no_exit_code() {
    let step = Step {
        name: "clippy".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["clippy".to_owned()],
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: None,
                stdout: "out line\n".to_owned(),
                stderr: "err line\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "clippy".to_owned(),
            verdict: Verdict::Failed {
                detail: "out line\nerr line".to_owned(),
            },
        }
    );
}

#[test]
fn joins_stdout_before_stderr_with_one_newline_when_both_streams_have_text() {
    let step = Step {
        name: "check".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: Vec::new(),
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(101),
                stdout: "first\nsecond".to_owned(),
                stderr: "third".to_owned(),
            })
        ),
        StepOutcome {
            name: "check".to_owned(),
            verdict: Verdict::Failed {
                detail: "first\nsecond\nthird".to_owned(),
            },
        }
    );
}

#[test]
fn uses_only_stdout_as_the_output_when_stderr_is_empty() {
    let step = Step {
        name: "check".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: Vec::new(),
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(2),
                stdout: "only out".to_owned(),
                stderr: String::new(),
            })
        ),
        StepOutcome {
            name: "check".to_owned(),
            verdict: Verdict::Failed {
                detail: "only out".to_owned(),
            },
        }
    );
}

#[test]
fn uses_only_stderr_as_the_output_when_stdout_is_empty() {
    let step = Step {
        name: "check".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: Vec::new(),
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(2),
                stdout: String::new(),
                stderr: "only err".to_owned(),
            })
        ),
        StepOutcome {
            name: "check".to_owned(),
            verdict: Verdict::Failed {
                detail: "only err".to_owned(),
            },
        }
    );
}

#[test]
fn fails_with_an_empty_detail_when_both_streams_are_empty() {
    let step = Step {
        name: "check".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: Vec::new(),
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(3),
                stdout: String::new(),
                stderr: String::new(),
            })
        ),
        StepOutcome {
            name: "check".to_owned(),
            verdict: Verdict::Failed {
                detail: String::new(),
            },
        }
    );
}

#[test]
fn trims_trailing_whitespace_from_each_stream_and_keeps_leading_whitespace() {
    let step = Step {
        name: "check".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: Vec::new(),
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(1),
                stdout: "  indented out\nsecond out   \n\n".to_owned(),
                stderr: "\t indented err \t\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "check".to_owned(),
            verdict: Verdict::Failed {
                detail: "  indented out\nsecond out\n\t indented err".to_owned(),
            },
        }
    );
}

#[test]
fn leaves_out_a_stream_that_is_only_whitespace() {
    let step = Step {
        name: "check".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: Vec::new(),
        },
        expectation: Expectation::ExitZero,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(1),
                stdout: "out".to_owned(),
                stderr: "   \n\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "check".to_owned(),
            verdict: Verdict::Failed {
                detail: "out".to_owned(),
            },
        }
    );
}

#[test]
fn passes_under_no_matches_when_the_exit_code_is_one() {
    let step = Step {
        name: "no-stubs".to_owned(),
        command_line: CommandLine {
            program: "rg".to_owned(),
            args: vec!["stub".to_owned()],
        },
        expectation: Expectation::NoMatches,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "searched 40 files\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "no-stubs".to_owned(),
            verdict: Verdict::Passed,
        }
    );
}

#[test]
fn fails_under_no_matches_with_the_trimmed_matches_and_no_stderr_when_the_exit_code_is_zero() {
    let step = Step {
        name: "no-stubs".to_owned(),
        command_line: CommandLine {
            program: "rg".to_owned(),
            args: vec!["stub".to_owned()],
        },
        expectation: Expectation::NoMatches,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(0),
                stdout: "src/a.rs:3\nsrc/b.rs:9  \n\n".to_owned(),
                stderr: "this must not appear\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "no-stubs".to_owned(),
            verdict: Verdict::Failed {
                detail: "src/a.rs:3\nsrc/b.rs:9".to_owned(),
            },
        }
    );
}

#[test]
fn fails_under_no_matches_with_the_output_when_the_exit_code_is_two() {
    let step = Step {
        name: "no-stubs".to_owned(),
        command_line: CommandLine {
            program: "rg".to_owned(),
            args: vec!["stub".to_owned()],
        },
        expectation: Expectation::NoMatches,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(2),
                stdout: "partial out\n".to_owned(),
                stderr: "bad pattern\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "no-stubs".to_owned(),
            verdict: Verdict::Failed {
                detail: "partial out\nbad pattern".to_owned(),
            },
        }
    );
}

#[test]
fn fails_under_no_matches_with_the_output_when_there_is_no_exit_code() {
    let step = Step {
        name: "no-stubs".to_owned(),
        command_line: CommandLine {
            program: "rg".to_owned(),
            args: vec!["stub".to_owned()],
        },
        expectation: Expectation::NoMatches,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: None,
                stdout: "partial out\n".to_owned(),
                stderr: "killed\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "no-stubs".to_owned(),
            verdict: Verdict::Failed {
                detail: "partial out\nkilled".to_owned(),
            },
        }
    );
}

#[test]
fn fails_under_pure_core_crates_with_the_output_when_the_exit_code_is_not_zero() {
    let step = Step {
        name: "purity".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["metadata".to_owned()],
        },
        expectation: Expectation::PureCoreCrates,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(101),
                stdout: PURE_METADATA.to_owned(),
                stderr: "error: failed to load manifest\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "purity".to_owned(),
            verdict: Verdict::Failed {
                detail: format!("{PURE_METADATA}\nerror: failed to load manifest"),
            },
        }
    );
}

#[test]
fn fails_under_pure_core_crates_with_the_metadata_error_text_when_stdout_is_not_valid_json() {
    let step = Step {
        name: "purity".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["metadata".to_owned()],
        },
        expectation: Expectation::PureCoreCrates,
    };

    let outcome = judge_step(
        &step,
        Ok(Captured {
            exit_code: Some(0),
            stdout: "this is not json".to_owned(),
            stderr: String::new(),
        }),
    );

    assert_eq!(outcome.name, "purity");
    match &outcome.verdict {
        Verdict::Failed { detail } => assert!(
            detail.starts_with("invalid cargo metadata: "),
            "detail did not start with the metadata error prefix: {detail:?}"
        ),
        Verdict::Passed => panic!("expected a failure, got {outcome:?}"),
    }
}

#[test]
fn passes_under_pure_core_crates_when_the_workspace_has_no_violation() {
    let step = Step {
        name: "purity".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["metadata".to_owned()],
        },
        expectation: Expectation::PureCoreCrates,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(0),
                stdout: PURE_METADATA.to_owned(),
                stderr: "    Finished\n".to_owned(),
            })
        ),
        StepOutcome {
            name: "purity".to_owned(),
            verdict: Verdict::Passed,
        }
    );
}

#[test]
fn fails_under_pure_core_crates_with_one_line_per_violation_in_order() {
    let step = Step {
        name: "purity".to_owned(),
        command_line: CommandLine {
            program: "cargo".to_owned(),
            args: vec!["metadata".to_owned()],
        },
        expectation: Expectation::PureCoreCrates,
    };

    assert_eq!(
        judge_step(
            &step,
            Ok(Captured {
                exit_code: Some(0),
                stdout: TWO_VIOLATIONS_METADATA.to_owned(),
                stderr: "this must not appear".to_owned(),
            })
        ),
        StepOutcome {
            name: "purity".to_owned(),
            verdict: Verdict::Failed {
                detail: concat!(
                    "pricing: core crate depends on reqwest, ",
                    "which is not a workspace core crate\n",
                    "app: no [package.metadata.standards] kind (core, adapter, app or tool)",
                )
                .to_owned(),
            },
        }
    );
}

proptest! {
    #[test]
    fn names_the_outcome_after_the_step_for_any_input(
        name in "[a-z-]{0,12}",
        expectation_index in 0_usize..3,
        exit_code in prop::option::of(-2_i32..3),
        stdout in "[^\u{0}]{0,32}",
        stderr in "[^\u{0}]{0,32}",
        started in any::<bool>(),
    ) {
        let step = Step {
            name: name.clone(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: Vec::new(),
            },
            expectation: match expectation_index {
                0 => Expectation::ExitZero,
                1 => Expectation::NoMatches,
                _ => Expectation::PureCoreCrates,
            },
        };
        let result = if started {
            Ok(Captured { exit_code, stdout, stderr })
        } else {
            Err(SpawnFailed {
                program: "cargo".to_owned(),
                reason: "program not found".to_owned(),
            })
        };

        prop_assert_eq!(judge_step(&step, result).name, name);
    }

    #[test]
    fn judges_arbitrary_stdout_under_pure_core_crates_without_panicking(
        stdout in ".*",
        exit_code in prop::option::of(-2_i32..3),
    ) {
        let step = Step {
            name: "purity".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: vec!["metadata".to_owned()],
            },
            expectation: Expectation::PureCoreCrates,
        };

        prop_assert_eq!(
            judge_step(&step, Ok(Captured { exit_code, stdout, stderr: String::new() })).name,
            "purity"
        );
    }

    #[test]
    fn passes_under_exit_zero_exactly_when_the_exit_code_is_zero(
        exit_code in prop::option::of(-2_i32..3),
        stdout in "[^\u{0}]{0,32}",
        stderr in "[^\u{0}]{0,32}",
    ) {
        let step = Step {
            name: "fmt".to_owned(),
            command_line: CommandLine {
                program: "cargo".to_owned(),
                args: vec!["fmt".to_owned()],
            },
            expectation: Expectation::ExitZero,
        };

        let outcome = judge_step(&step, Ok(Captured { exit_code, stdout, stderr }));

        prop_assert_eq!(outcome.verdict == Verdict::Passed, exit_code == Some(0));
    }
}
