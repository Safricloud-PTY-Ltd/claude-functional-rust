//! Pure logic: what to run, how to judge it, how to report it.

mod check_steps;
mod describe_violation;
mod find_purity_violations;
mod judge_step;
mod parse_command;
mod parse_metadata;
mod render_report;

pub use check_steps::check_steps;
pub use describe_violation::describe_violation;
pub use find_purity_violations::find_purity_violations;
pub use judge_step::judge_step;
pub use parse_command::parse_command;
pub use parse_metadata::parse_metadata;
pub use render_report::render_report;
