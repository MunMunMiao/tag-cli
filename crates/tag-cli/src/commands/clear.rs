use crate::cli::ClearArgs;
use tag_core::error::TagCliError;
use tag_core::workflow::builder::WorkflowBuilder;
use tag_core::workflow::context::{Context, CoverAction, TagUpdates};
use tag_core::workflow::steps::detect_format::DetectAudioFormatStep;
use tag_core::workflow::steps::read_metadata::ReadMetadataStep;
use tag_core::workflow::steps::resolve_output::ResolveOutputPathStep;
use tag_core::workflow::steps::save_file::{SaveFileStep, SaveMode};
use tag_core::workflow::steps::update_tags::UpdateTagsStep;

pub fn run(args: &ClearArgs, verbose: bool) -> Result<(), TagCliError> {
    let mut ctx = Context::new(&args.input, args.dry_run, verbose);

    let updates = if args.all {
        TagUpdates {
            sets: Default::default(),
            clears: vec![],
            clear_all: true,
            replace: false,
        }
    } else {
        TagUpdates {
            sets: Default::default(),
            clears: args.keys.clone(),
            clear_all: false,
            replace: false,
        }
    };

    if args.all {
        ctx.cover_action = CoverAction::Clear;
    }

    let workflow = WorkflowBuilder::new()
        .add(Box::new(ReadMetadataStep::new()))
        .add(Box::new(DetectAudioFormatStep::new()))
        .add(Box::new(UpdateTagsStep::new(updates)))
        .add(Box::new(ResolveOutputPathStep::new(
            args.output.clone(),
            !crate::cli::Cli::is_confirmed(args.yes),
            "tag-cli clear",
        )))
        .add(Box::new(SaveFileStep::new(SaveMode::Incremental)))
        .build();

    workflow.run(&mut ctx)?;
    if args.dry_run
        && let Some(diff) = crate::diff::compute_diff(&ctx)
    {
        println!("{}", diff);
    }
    for msg in &ctx.report.messages {
        crate::report::status(msg);
    }
    Ok(())
}
