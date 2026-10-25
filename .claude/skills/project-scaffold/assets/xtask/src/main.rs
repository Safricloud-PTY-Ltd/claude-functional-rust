//! Entry point for `cargo xtask`: the composition root. Builds the real process runner, hands
//! the arguments to [`xtask::run`], writes what it returns.

use std::ffi::OsString;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use xtask::{SystemRunCommand, run};

fn main() -> ExitCode {
    let args: Result<Vec<String>, OsString> = std::env::args_os()
        .skip(1)
        .map(OsString::into_string)
        .collect();
    let (report, exit_code) = match args {
        Ok(args) => {
            // Steps must run from the workspace root wherever `cargo xtask` was invoked from.
            let workspace_root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
            let output = run(&args, &SystemRunCommand::new(workspace_root));
            (output.report, output.exit_code)
        }
        Err(arg) => (
            format!(
                "argument is not valid Unicode: {}\nusage: cargo xtask check\n",
                arg.to_string_lossy()
            ),
            2,
        ),
    };
    // A report that can't be written must not pass for a green check.
    match std::io::stdout().lock().write_all(report.as_bytes()) {
        Ok(()) => ExitCode::from(exit_code),
        Err(_) => ExitCode::FAILURE,
    }
}
