//! The workspace's own tooling. `cargo xtask check` runs every quality gate in order, judges
//! each one, prints a report, and exits non-zero if any gate failed.
//!
//! Pure logic lives in `core`; the one effect, starting a process, lives in `shell` behind the
//! [`RunCommand`] port.
#![forbid(unsafe_code)]

mod core;
mod run;
mod shell;
mod types;

pub use crate::core::{
    check_steps, describe_violation, find_purity_violations, judge_step, parse_command,
    parse_metadata, render_report,
};
pub use crate::run::run;
pub use crate::shell::{FakeRunCommand, SystemRunCommand};
pub use crate::types::{
    Captured, Command, CommandLine, CrateKind, Dependency, DependencyScope, Expectation,
    InvalidMetadata, Package, PurityViolation, RunCommand, RunOutput, SpawnFailed, Step,
    StepOutcome, UnknownCommand, Verdict,
};
