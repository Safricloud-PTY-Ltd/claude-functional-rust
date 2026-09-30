//! The values, error kinds and ports of the `xtask` crate.
//!
//! These are boundary data: their shape is the contract, so their fields are public.

use std::fmt;

/// A command `cargo xtask` understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Run every quality gate in order and report.
    Check,
}

/// A program and its arguments, exactly as they are handed to the operating system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    /// The program to start, resolved through `PATH`.
    pub program: String,
    /// Its arguments, one per element, with no shell quoting.
    pub args: Vec<String>,
}

/// What a finished process left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    /// The exit code, or `None` when the process was ended by a signal.
    pub exit_code: Option<i32>,
    /// Everything it wrote to standard output, lossily decoded as UTF-8.
    pub stdout: String,
    /// Everything it wrote to standard error, lossily decoded as UTF-8.
    pub stderr: String,
}

/// How a step's captured result is turned into a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expectation {
    /// The step passes when the process exits with code 0.
    ExitZero,
    /// The step is a search: it passes when the process exits with code 1, meaning it found
    /// nothing. Exit code 0 means it found something, and standard output lists what.
    NoMatches,
    /// The step's standard output is `cargo metadata` JSON: it passes when the process exits
    /// with code 0 and the workspace it describes has no [`PurityViolation`].
    PureCoreCrates,
}

/// One quality gate: what to run and how to judge it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// A short unique name for the report, lowercase with hyphens: `fmt`, `no-stubs`.
    pub name: String,
    /// What to run.
    pub command_line: CommandLine,
    /// How to judge what it produced.
    pub expectation: Expectation,
}

/// Whether a step passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The step met its expectation.
    Passed,
    /// The step did not meet its expectation.
    Failed {
        /// What to show the developer. May span several lines; may be empty.
        detail: String,
    },
}

/// A step's name and its verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepOutcome {
    /// The name of the [`Step`] that was judged.
    pub name: String,
    /// Whether it passed.
    pub verdict: Verdict,
}

/// What the process should print and exit with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    /// The process exit code.
    pub exit_code: u8,
    /// The text for standard output, ending in a newline.
    pub report: String,
}

/// What role a workspace crate plays, from `[package.metadata.standards] kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateKind {
    /// Pure logic. Depends at runtime on workspace core crates only.
    Core,
    /// One external concern behind a port.
    Adapter,
    /// A composition root.
    App,
    /// Developer tooling.
    Tool,
}

/// When a dependency is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyScope {
    /// Under `[dependencies]`: linked into the crate.
    Runtime,
    /// Under `[dev-dependencies]`: tests, examples and benches only.
    Dev,
    /// Under `[build-dependencies]`: the build script only.
    Build,
}

/// One declared dependency of a workspace crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The package name of the dependency.
    pub name: String,
    /// When it is needed.
    pub scope: DependencyScope,
}

/// One workspace crate, as far as the purity rule cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// The package name.
    pub name: String,
    /// Its declared kind, or `None` when the key is missing or is not a recognised kind.
    pub kind: Option<CrateKind>,
    /// Every dependency it declares, in the order `cargo metadata` lists them.
    pub dependencies: Vec<Dependency>,
}

/// A way the workspace breaks the crate-purity rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurityViolation {
    /// The crate declares no kind, or one that is not recognised.
    MissingKind {
        /// The offending package.
        package: String,
    },
    /// A core crate has a runtime dependency that is not a workspace core crate.
    ImpureDependency {
        /// The core package.
        package: String,
        /// The dependency it may not have.
        dependency: String,
    },
}

/// The first argument was missing or is not a known command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCommand {
    /// The argument as given, or `None` when there was none.
    pub given: Option<String>,
}

/// A process could not be started at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnFailed {
    /// The program that could not be started.
    pub program: String,
    /// The operating system's explanation.
    pub reason: String,
}

/// Text that should have been `cargo metadata` JSON was not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidMetadata {
    /// The parser's explanation.
    pub reason: String,
}

/// The port through which this crate starts processes.
pub trait RunCommand {
    /// Runs a command line to completion and captures what it produced.
    ///
    /// # Errors
    /// - [`SpawnFailed`] when the process could not be started. A process that starts and then
    ///   exits with a non-zero code is not an error; it is a [`Captured`].
    fn run(&self, command_line: &CommandLine) -> Result<Captured, SpawnFailed>;
}

impl fmt::Display for UnknownCommand {
    /// Writes `unknown command: <given>`, or `no command given` when `given` is `None`.
    /// No trailing newline.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.given {
            Some(given) => write!(f, "unknown command: {given}"),
            None => write!(f, "no command given"),
        }
    }
}

impl fmt::Display for SpawnFailed {
    /// Writes `could not start <program>: <reason>`. No trailing newline.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not start {}: {}", self.program, self.reason)
    }
}

impl fmt::Display for InvalidMetadata {
    /// Writes `invalid cargo metadata: <reason>`. No trailing newline.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid cargo metadata: {}", self.reason)
    }
}

impl std::error::Error for UnknownCommand {}
impl std::error::Error for SpawnFailed {}
impl std::error::Error for InvalidMetadata {}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "types_tests.rs"]
mod tests;
