use crate::cli::{CoverArgs, CoverCommands};
use tag_core::error::TagCliError;
use tag_core::workflow::builder::WorkflowBuilder;
use tag_core::workflow::context::{Context, CoverAction};
use tag_core::workflow::steps::detect_format::DetectAudioFormatStep;
use tag_core::workflow::steps::process_cover::ProcessCoverStep;
use tag_core::workflow::steps::read_metadata::ReadMetadataStep;
use tag_core::workflow::steps::resolve_output::ResolveOutputPathStep;
use tag_core::workflow::steps::save_file::{SaveFileStep, SaveMode};
use tag_core::workflow::steps::update_cover::UpdateCoverStep;

pub fn run(args: &CoverArgs, verbose: bool) -> Result<(), TagCliError> {
    match &args.command {
        CoverCommands::Get(a) => cover_get(a),
        CoverCommands::Set(a) => cover_set(a, verbose),
        CoverCommands::Clear(a) => cover_clear(a, verbose),
    }
}

fn cover_get(args: &crate::cli::CoverGetArgs) -> Result<(), TagCliError> {
    let metadata = tag_core::taglib::read_metadata_from_path(&args.input)?;
    let front_cover = metadata.pictures.iter().find(|p| {
        p.picture_type
            .as_deref()
            .unwrap_or("")
            .eq_ignore_ascii_case("Front Cover")
    });
    let picture = match &args.picture_type {
        Some(t) => metadata.pictures.iter().find(|p| {
            p.picture_type
                .as_deref()
                .unwrap_or("")
                .eq_ignore_ascii_case(t)
        }),
        None => front_cover.or_else(|| metadata.pictures.first()),
    }
    .ok_or_else(|| TagCliError::ImageProcessingError("no cover found".to_string()))?;

    std::fs::write(&args.output, &picture.data).map_err(TagCliError::Io)?;

    crate::report::status(format!(
        "cover saved to {} ({} bytes)",
        args.output.display(),
        picture.data.len()
    ));
    Ok(())
}

fn cover_set(args: &crate::cli::CoverSetArgs, verbose: bool) -> Result<(), TagCliError> {
    let mut ctx = Context::new(&args.input, args.dry_run, verbose);
    ctx.cover_action = CoverAction::Set(args.image.clone());

    let mut config = args.image_options.to_image_processing_config()?;
    config.picture_type = args.picture_type.clone();

    let workflow = WorkflowBuilder::new()
        .add(Box::new(ReadMetadataStep::new()))
        .add(Box::new(DetectAudioFormatStep::new()))
        .add(Box::new(ProcessCoverStep::new(config)))
        .add(Box::new(UpdateCoverStep::new()))
        .add(Box::new(ResolveOutputPathStep::new(
            args.output.clone(),
            !crate::cli::Cli::is_confirmed(args.yes),
            "tag-cli cover set",
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

fn cover_clear(args: &crate::cli::CoverClearArgs, verbose: bool) -> Result<(), TagCliError> {
    let mut ctx = Context::new(&args.input, args.dry_run, verbose);
    ctx.cover_action = CoverAction::Clear;

    let workflow = WorkflowBuilder::new()
        .add(Box::new(ReadMetadataStep::new()))
        .add(Box::new(ResolveOutputPathStep::new(
            args.output.clone(),
            !crate::cli::Cli::is_confirmed(args.yes),
            "tag-cli cover clear",
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
