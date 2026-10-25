//! Black-box tests of the `xtask` crate's public API.
#![allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use proptest::prelude::*;
use xtask::{
    Captured, CommandLine, Expectation, FakeRunCommand, RunCommand, SpawnFailed, Step, check_steps,
    run,
};

/// A workspace `cargo metadata --format-version 1 --no-deps` would describe when it is pure:
/// every crate declares a kind, and the core crates take runtime dependencies on workspace core
/// crates only. `orders`'s dev dependency on a third-party crate is allowed, and the tool crate may
/// depend on anything.
const PURE_METADATA: &str = r#"{
  "packages": [
    {
      "name": "shared",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": []
    },
    {
      "name": "orders",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": [
        { "name": "shared", "kind": null },
        { "name": "proptest", "kind": "dev" }
      ]
    },
    {
      "name": "xtask",
      "metadata": { "standards": { "kind": "tool" } },
      "dependencies": [{ "name": "serde_json", "kind": null }]
    }
  ]
}"#;

/// The same workspace, except that the core crate `orders` has taken a runtime dependency on the
/// third-party crate `serde_json`.
const IMPURE_METADATA: &str = r#"{
  "packages": [
    {
      "name": "shared",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": []
    },
    {
      "name": "orders",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": [
        { "name": "shared", "kind": null },
        { "name": "serde_json", "kind": null }
      ]
    }
  ]
}"#;

/// A workspace whose crate `mystery` declares no `[package.metadata.standards] kind`.
const KINDLESS_METADATA: &str = r#"{
  "packages": [
    {
      "name": "shared",
      "metadata": { "standards": { "kind": "core" } },
      "dependencies": []
    },
    {
      "name": "mystery",
      "metadata": null,
      "dependencies": [{ "name": "shared", "kind": null }]
    }
  ]
}"#;

/// One line of the stub search's standard output: a contract whose body was never written. Kept
/// in a string literal that starts mid-line so the search this crate defines does not match this
/// file.
const LEFTOVER_MATCH: &str =
    "crates/orders/src/parse_header.rs:31:    stub!(\"parse_header\", bytes)";

/// Builds a captured result of a process that exited with `exit_code`, wrote `stdout`, and wrote
/// nothing to standard error.
fn captured(exit_code: i32, stdout: &str) -> Captured {
    Captured {
        exit_code: Some(exit_code),
        stdout: stdout.to_owned(),
        stderr: String::new(),
    }
}

/// Answers one step the way a clean workspace would: the exit code its expectation calls a pass,
/// and for the purity step `metadata` on standard output.
fn favourable_answer(step: &Step, metadata: &str) -> Captured {
    match step.expectation {
        Expectation::ExitZero => captured(0, ""),
        Expectation::NoMatches => captured(1, ""),
        Expectation::PureCoreCrates => captured(0, metadata),
    }
}

/// The fake's table for a run in which every step is answered favourably, the purity step with
/// `metadata`.
fn favourable_table(metadata: &str) -> Vec<(CommandLine, Result<Captured, SpawnFailed>)> {
    check_steps()
        .into_iter()
        .map(|step| {
            let answer = favourable_answer(&step, metadata);
            (step.command_line, Ok(answer))
        })
        .collect()
}

/// The same table, except that the step called `name` is answered with `answer`.
fn table_answering(
    metadata: &str,
    name: &str,
    answer: &Result<Captured, SpawnFailed>,
) -> Vec<(CommandLine, Result<Captured, SpawnFailed>)> {
    check_steps()
        .into_iter()
        .map(|step| {
            let chosen = if step.name == name {
                answer.clone()
            } else {
                Ok(favourable_answer(&step, metadata))
            };
            (step.command_line, chosen)
        })
        .collect()
}

/// The arguments of `cargo xtask check`.
fn check_args() -> Vec<String> {
    vec![String::from("check")]
}

/// A [`RunCommand`] written outside the crate: it starts nothing and reports every command line
/// as having exited quietly with code 0.
struct QuietSuccess;

impl RunCommand for QuietSuccess {
    fn run(&self, _command_line: &CommandLine) -> Result<Captured, SpawnFailed> {
        Ok(Captured {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

/// A runner that answers the `n`th call as the `n`th step expects, counts its calls, and notes
/// whether any call was not the next step's command line. The fake is a stateless table, so it
/// can't see a step that runs twice.
struct CountingRunner {
    steps: Vec<Step>,
    calls: AtomicUsize,
    out_of_order: AtomicBool,
}

impl RunCommand for CountingRunner {
    fn run(&self, command_line: &CommandLine) -> Result<Captured, SpawnFailed> {
        let step = self.steps.get(self.calls.fetch_add(1, Ordering::SeqCst));
        if step.map(|step| &step.command_line) != Some(command_line) {
            self.out_of_order.store(true, Ordering::SeqCst);
        }
        step.map_or_else(
            || QuietSuccess.run(command_line),
            |step| Ok(favourable_answer(step, PURE_METADATA)),
        )
    }
}

#[test]
fn reports_every_step_passed_and_exits_zero_when_the_workspace_is_clean() {
    let runner = FakeRunCommand::new(favourable_table(PURE_METADATA));

    let output = run(&check_args(), &runner);

    assert_eq!(
        output.report,
        concat!(
            "ok    fmt\n",
            "ok    clippy\n",
            "ok    test\n",
            "ok    doctest\n",
            "ok    deny\n",
            "ok    machete\n",
            "ok    no-stubs\n",
            "ok    purity\n",
            "check: 8 passed, 0 failed\n",
        )
    );
    assert!(output.report.ends_with("check: 8 passed, 0 failed\n"));
    assert_eq!(output.exit_code, 0);
}

#[test]
fn fails_the_purity_step_naming_the_core_crate_and_its_third_party_dependency() {
    let runner = FakeRunCommand::new(favourable_table(IMPURE_METADATA));

    let output = run(&check_args(), &runner);

    assert_eq!(
        output.report,
        concat!(
            "ok    fmt\n",
            "ok    clippy\n",
            "ok    test\n",
            "ok    doctest\n",
            "ok    deny\n",
            "ok    machete\n",
            "ok    no-stubs\n",
            "FAIL  purity\n",
            "    orders: core crate depends on serde_json, which is not a workspace core crate\n",
            "check: 7 passed, 1 failed\n",
        )
    );
    assert_eq!(output.exit_code, 1);
}

#[test]
fn fails_the_purity_step_when_a_workspace_crate_declares_no_kind() {
    let runner = FakeRunCommand::new(favourable_table(KINDLESS_METADATA));

    let output = run(&check_args(), &runner);

    assert_eq!(
        output.report,
        concat!(
            "ok    fmt\n",
            "ok    clippy\n",
            "ok    test\n",
            "ok    doctest\n",
            "ok    deny\n",
            "ok    machete\n",
            "ok    no-stubs\n",
            "FAIL  purity\n",
            "    mystery: no [package.metadata.standards] kind (core, adapter, app or tool)\n",
            "check: 7 passed, 1 failed\n",
        )
    );
    assert_eq!(output.exit_code, 1);
}

#[test]
fn fails_the_stub_search_with_the_leftover_stub_indented_in_the_report() {
    let found = format!("{LEFTOVER_MATCH}\n");
    let runner = FakeRunCommand::new(table_answering(
        PURE_METADATA,
        "no-stubs",
        &Ok(captured(0, &found)),
    ));

    let output = run(&check_args(), &runner);

    let indented = format!("    {LEFTOVER_MATCH}\n");
    let expected = [
        "ok    fmt\n",
        "ok    clippy\n",
        "ok    test\n",
        "ok    doctest\n",
        "ok    deny\n",
        "ok    machete\n",
        "FAIL  no-stubs\n",
        indented.as_str(),
        "ok    purity\n",
        "check: 7 passed, 1 failed\n",
    ]
    .concat();
    assert_eq!(output.report, expected);
    assert_eq!(output.exit_code, 1);
}

#[test]
fn fails_only_the_step_whose_tool_is_not_installed_and_says_could_not_start() {
    let missing = Err(SpawnFailed {
        program: String::from("cargo"),
        reason: String::from("program not found"),
    });
    let runner = FakeRunCommand::new(table_answering(PURE_METADATA, "machete", &missing));

    let output = run(&check_args(), &runner);

    assert_eq!(
        output.report,
        concat!(
            "ok    fmt\n",
            "ok    clippy\n",
            "ok    test\n",
            "ok    doctest\n",
            "ok    deny\n",
            "FAIL  machete\n",
            "    could not start cargo: program not found\n",
            "ok    no-stubs\n",
            "ok    purity\n",
            "check: 7 passed, 1 failed\n",
        )
    );
    assert_eq!(output.exit_code, 1);
}

#[test]
fn returns_exit_code_two_and_the_usage_line_when_the_command_is_unknown() {
    let runner = FakeRunCommand::new(Vec::new());

    let output = run(&[String::from("build")], &runner);

    assert_eq!(
        output.report,
        "unknown command: build\nusage: cargo xtask check\n"
    );
    assert_eq!(output.exit_code, 2);
}

#[test]
fn returns_exit_code_two_and_the_usage_line_when_no_command_is_given() {
    let runner = FakeRunCommand::new(Vec::new());

    let output = run(&[], &runner);

    assert_eq!(
        output.report,
        "no command given\nusage: cargo xtask check\n"
    );
    assert_eq!(output.exit_code, 2);
}

#[test]
fn accepts_a_run_command_implemented_outside_the_crate() {
    let runner = QuietSuccess;

    let output = run(&check_args(), &runner);

    assert!(
        output.report.contains("FAIL  purity"),
        "empty standard output is not cargo metadata, so the purity step must fail: {}",
        output.report
    );
    assert_eq!(output.exit_code, 1);
}

#[test]
fn check_calls_the_runner_exactly_once_per_step_in_step_order() {
    let runner = CountingRunner {
        steps: check_steps(),
        calls: AtomicUsize::new(0),
        out_of_order: AtomicBool::new(false),
    };

    let output = run(&check_args(), &runner);

    assert_eq!(output.exit_code, 0, "{}", output.report);
    assert_eq!(runner.calls.load(Ordering::SeqCst), check_steps().len());
    assert!(!runner.out_of_order.load(Ordering::SeqCst));
}

proptest! {
    /// Every argument list whose first element is not exactly `check` is an unknown command, so
    /// the usage line is the last thing printed and the exit code is 2.
    #[test]
    fn returns_exit_code_two_for_any_first_argument_that_is_not_check(
        args in proptest::collection::vec(".*", 0..4_usize),
    ) {
        prop_assume!(args.first().map(String::as_str) != Some("check"));
        let runner = FakeRunCommand::new(Vec::new());

        let output = run(&args, &runner);

        prop_assert_eq!(output.exit_code, 2);
        prop_assert!(output.report.ends_with("usage: cargo xtask check\n"));
    }
}
