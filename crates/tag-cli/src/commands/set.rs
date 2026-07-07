use std::collections::BTreeMap;

use crate::cli::SetArgs;
use crate::commands::run_and_report;
use tag_core::error::TagCliError;
use tag_core::workflow::builder::WorkflowBuilder;
use tag_core::workflow::context::{Context, TagUpdates};
use tag_core::workflow::steps::detect_format::DetectAudioFormatStep;
use tag_core::workflow::steps::read_metadata::ReadMetadataStep;
use tag_core::workflow::steps::resolve_output::ResolveOutputPathStep;
use tag_core::workflow::steps::save_file::{SaveFileStep, SaveMode};
use tag_core::workflow::steps::update_tags::UpdateTagsStep;

pub fn run(args: &SetArgs, verbose: bool) -> Result<(), TagCliError> {
    let mut ctx = Context::new(&args.input, args.dry_run, verbose);

    let mut sets = BTreeMap::new();
    for (k, v) in &args.tags {
        sets.entry(k.clone())
            .or_insert_with(Vec::new)
            .push(v.clone());
    }
    let updates = TagUpdates {
        sets,
        clears: vec![],
        clear_all: false,
        replace: args.replace,
    };

    let save_mode = if args.replace {
        SaveMode::FullReplace
    } else {
        SaveMode::Incremental
    };

    let workflow = WorkflowBuilder::new()
        .add(Box::new(ReadMetadataStep::new()))
        .add(Box::new(DetectAudioFormatStep::new()))
        .add(Box::new(UpdateTagsStep::new(updates)))
        .add(Box::new(ResolveOutputPathStep::new(
            args.output.clone(),
            !crate::cli::Cli::is_confirmed(args.yes),
            "tag-cli set",
        )))
        .add(Box::new(SaveFileStep::new(save_mode)))
        .build();

    run_and_report(&mut ctx, workflow, args.dry_run)
}
