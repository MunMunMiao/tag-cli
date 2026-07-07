use tag_core::error::TagCliError;
use tag_core::workflow::Workflow;
use tag_core::workflow::context::Context;

pub mod apply;
pub mod clear;
pub mod cover;
pub mod export_metadata;
pub mod get;
pub mod info;
pub mod list_keys;
pub mod set;
pub mod update;

/// Write a status message to stderr.
///
/// Status messages are human-facing progress feedback and should not pollute
/// stdout, where structured data and `--dry-run` diffs are written.
pub fn status(msg: impl AsRef<str>) {
    eprintln!("{}", msg.as_ref());
}

/// Run a workflow and emit the dry-run diff plus any report messages.
pub fn run_and_report(
    ctx: &mut Context,
    workflow: Workflow,
    dry_run: bool,
) -> Result<(), TagCliError> {
    workflow.run(ctx)?;
    if dry_run && let Some(diff) = crate::diff::compute_diff(ctx) {
        println!("{}", diff);
    }
    for msg in &ctx.report.messages {
        eprintln!("{}", msg);
    }
    Ok(())
}
