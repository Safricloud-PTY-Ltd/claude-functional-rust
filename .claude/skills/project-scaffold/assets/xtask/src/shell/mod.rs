//! The one effect this crate has: starting a process. The real adapter and its fake.

mod fake_run_command;
mod system_run_command;

pub use fake_run_command::FakeRunCommand;
pub use system_run_command::SystemRunCommand;
