use crate::types::{CommandLine, Expectation, Step};

/// Step 3's *filter*: the normal suite, as the table on [`check_steps`] spells it.
const NORMAL_SUITE_FILTER: &str = "not (test(/(^|::)(slow|perf)_tests::/) | (kind(test) & binary(/_(slow|perf)$|^xtask$/) & not binary(=xtask)))";

/// Step 3's arguments, as the table on [`check_steps`] spells them.
const TEST_ARGS: [&str; 7] = [
    "nextest",
    "run",
    "--workspace",
    "--no-tests=pass",
    "--ignore-default-filter",
    "-E",
    NORMAL_SUITE_FILTER,
];

/// Step 7's arguments, as the table on [`check_steps`] spells them.
const NO_STUBS_ARGS: [&str; 9] = [
    "grep",
    "--untracked",
    "-n",
    "-E",
    r"^[[:space:]]*stub!\(",
    "--",
    "*.rs",
    ":!crates/shared",
    ":!.claude",
];

/// Lists the quality gates `cargo xtask check` runs, in the order it runs them.
///
/// # Returns
/// Exactly these eight steps, in this order. Every program is `cargo` except the sixth, which
/// is `cargo-machete`, and the seventh, which is `git`. Arguments are one element each, exactly
/// as written, with no shell quoting.
///
/// | # | name | arguments | expectation |
/// | - | :-- | :-- | :-- |
/// | 1 | `fmt` | `fmt` `--all` `--check` | `ExitZero` |
/// | 2 | `clippy` | `clippy` `--workspace` `--all-targets` `--` `-D` `warnings` | `ExitZero` |
/// | 3 | `test` | `nextest` `run` `--workspace` `--no-tests=pass` `--ignore-default-filter` `-E` *filter* | `ExitZero` |
/// | 4 | `doctest` | `test` `--workspace` `--doc` | `ExitZero` |
/// | 5 | `deny` | `deny` `check` | `ExitZero` |
/// | 6 | `machete` | *(none: the argument vector is empty)* | `ExitZero` |
/// | 7 | `no-stubs` | `grep` `--untracked` `-n` `-E` *pattern* `--` `*.rs` `:!crates/shared` `:!.claude` | `NoMatches` |
/// | 8 | `purity` | `metadata` `--format-version` `1` `--no-deps` | `PureCoreCrates` |
///
/// The *filter* of step 3 is the nextest filter set that selects the normal suite,
/// `not (test(/(^|::)(slow|perf)_tests::/) | (kind(test) & binary(/_(slow|perf)$|^xtask$/) & not binary(=xtask)))`:
/// every test except those in a `slow_tests` or `perf_tests` module and those in an integration
/// test binary whose name ends in `_slow` or `_perf` (code-standards, Tests). It is the
/// `default-filter` of `[profile.default]` in `.config/nextest.toml`, character for character.
///
/// Why `^xtask$` is in the `binary()` and then taken out again: nextest refuses a `binary()`
/// matcher that matches no binary in the workspace, and `-E` is checked like the config. A
/// workspace with no `tests/<name>_slow.rs` or `tests/<name>_perf.rs` yet would fail the step
/// on `binary(/_(slow|perf)$/)` alone. `xtask` is the one binary this workspace always has, so
/// it keeps the matcher non-empty, and `not binary(=xtask)` removes it from the result.
///
/// `--ignore-default-filter` switches the config's `default-filter` off, so the step's
/// selection is exactly the normal suite whatever the config says: `-E` alone would be
/// intersected with the config's filter, and a narrower one would shrink the gate without a
/// word.
///
/// The *pattern* of step 7 is the POSIX extended regular expression
/// `^[[:space:]]*stub!\(` (caret, the bracket expression `[[:space:]]`, star, the letters
/// `stub`, an exclamation mark, a backslash, an opening parenthesis): a `stub!` call at the
/// start of a line, which is how every contract's stub body is written. Anchoring to the start
/// of the line is what keeps this file, and any test that mentions the macro inside a string,
/// from matching itself. `--untracked` also searches files not yet added (ignored files such as
/// `target/` stay out), since a new contract file is untracked mid-phase. `.claude` is excluded
/// because the scaffold skill keeps a template copy of `crates/shared` there, which is not part
/// of the workspace.
///
/// Step 6 starts the `cargo-machete` binary itself, not `cargo machete`: the tool decides
/// whether cargo invoked it by looking for `CARGO_PKG_NAME` in its environment, which
/// `cargo run` always sets, so under `cargo xtask` the subcommand form misreads its own name
/// as a directory to scan.
///
/// # Remarks
/// Pure and constant: every call returns an equal value. Step names are unique. Cheap gates
/// come first so a formatting slip fails in a second, not after the test run.
#[must_use]
pub fn check_steps() -> Vec<Step> {
    let zero = Expectation::ExitZero;
    let pure = Expectation::PureCoreCrates;
    let clippy_args: &[&str] = &[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ];
    let purity_args: &[&str] = &["metadata", "--format-version", "1", "--no-deps"];

    let table: [(&str, &str, &[&str], Expectation); 8] = [
        ("fmt", "cargo", &["fmt", "--all", "--check"], zero),
        ("clippy", "cargo", clippy_args, zero),
        ("test", "cargo", &TEST_ARGS, zero),
        ("doctest", "cargo", &["test", "--workspace", "--doc"], zero),
        ("deny", "cargo", &["deny", "check"], zero),
        ("machete", "cargo-machete", &[], zero),
        ("no-stubs", "git", &NO_STUBS_ARGS, Expectation::NoMatches),
        ("purity", "cargo", purity_args, pure),
    ];

    table
        .into_iter()
        .map(|(name, program, args, expectation)| Step {
            name: name.to_owned(),
            command_line: CommandLine {
                program: program.to_owned(),
                args: args.iter().copied().map(String::from).collect(),
            },
            expectation,
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "check_steps_tests.rs"]
mod tests;
