//! Entry point for `cargo xtask`: the composition root. Builds the real process runner, hands
//! the arguments to [`xtask::run`], prints what it returns.

use std::path::PathBuf;
use std::process::ExitCode;

use xtask::{SystemRunCommand, run};

#[allow(
    clippy::print_stdout,
    reason = "the composition root is the one place that prints"
)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Steps must run from the workspace root wherever `cargo xtask` was invoked from.
    let workspace_root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let output = run(&args, &SystemRunCommand::new(workspace_root));
    print!("{}", output.report);
    ExitCode::from(output.exit_code)
}
