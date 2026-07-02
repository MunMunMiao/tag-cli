use std::collections::BTreeMap;
use std::path::Path;

use crate::cli::ApplyArgs;
use tag_core::config::{ImageProcessing, Manifest};
use tag_core::error::TagCliError;
use tag_core::workflow::builder::WorkflowBuilder;
use tag_core::workflow::context::{
    Context, CoverAction, ImageProcessingConfig, ImageTargetFormat, TagUpdates,
};
use tag_core::workflow::steps::detect_format::DetectAudioFormatStep;
use tag_core::workflow::steps::process_cover::ProcessCoverStep;
use tag_core::workflow::steps::read_metadata::ReadMetadataStep;
use tag_core::workflow::steps::resolve_output::ResolveOutputPathStep;
use tag_core::workflow::steps::save_file::{SaveFileStep, SaveMode};
use tag_core::workflow::steps::update_cover::UpdateCoverStep;
use tag_core::workflow::steps::update_tags::UpdateTagsStep;

pub fn run(args: &ApplyArgs, verbose: bool) -> Result<(), TagCliError> {
    let manifest = Manifest::load(&args.filename)?;
    let manifest_dir = args
        .filename
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let image_config = build_image_config(&args.image_options, manifest.image_processing.as_ref())?;

    let mut successes = Vec::new();
    let mut skipped = Vec::new();
    let mut failures = Vec::new();

    for entry in manifest.expand_files(&manifest_dir)? {
        let input_path = manifest.resolve_file_path(&entry.path, &manifest_dir);
        let cover_path = entry
            .cover
            .as_ref()
            .map(|c| manifest.resolve_file_path(c, &manifest_dir));

        let mut ctx = Context::new(&input_path, args.dry_run, verbose);

        let mut sets = BTreeMap::new();
        for (k, v) in &manifest.defaults {
            sets.entry(k.clone())
                .or_insert_with(Vec::new)
                .push(v.clone());
        }
        // File-level tags override defaults.
        for (k, v) in &entry.tags {
            sets.insert(k.clone(), vec![v.clone()]);
        }

        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: true,
        });

        if let Some(cover) = cover_path {
            ctx.cover_action = CoverAction::Set(cover);
        }

        let mut builder = WorkflowBuilder::new()
            .add(Box::new(ReadMetadataStep::new()))
            .add(Box::new(DetectAudioFormatStep::new()));

        if matches!(ctx.cover_action, CoverAction::Set(_)) {
            let mut file_image_config = image_config.clone();
            file_image_config.picture_type = entry.picture_type.clone();
            builder = builder.add(Box::new(ProcessCoverStep::new(file_image_config)));
            builder = builder.add(Box::new(UpdateCoverStep::new()));
        }

        builder = builder
            .add(Box::new(UpdateTagsStep::new(
                ctx.tag_updates.take().expect("tag updates must be set"),
            )))
            .add(Box::new(ResolveOutputPathStep::new(
                None,
                !crate::cli::Cli::is_confirmed(args.yes),
                "tag-cli apply",
            )))
            .add(Box::new(SaveFileStep::new(SaveMode::FullReplace)));

        let workflow = builder.build();

        match workflow.run(&mut ctx) {
            Ok(()) => {
                if args.dry_run {
                    let _ = crate::diff::compute_diff(&ctx).map(|diff| println!("{}", diff));
                    for msg in &ctx.report.messages {
                        eprintln!("{}", msg);
                    }
                    skipped.push(input_path);
                } else {
                    successes.push(input_path);
                }
            }
            Err(e) => {
                failures.push((input_path, e.to_string()));
                if args.fail_fast {
                    break;
                }
            }
        }
    }

    crate::report::status(format!("Success: {}", successes.len()));
    for p in &successes {
        crate::report::status(format!("  ok {}", p.display()));
    }
    crate::report::status(format!("Skipped: {}", skipped.len()));
    for p in &skipped {
        crate::report::status(format!("  skip {} (dry-run)", p.display()));
    }
    crate::report::status(format!("Failures: {}", failures.len()));
    for (p, e) in &failures {
        crate::report::status(format!("  err {}: {}", p.display(), e));
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(TagCliError::ApplyFailed(format!(
            "{} file(s) failed to apply",
            failures.len()
        )))
    }
}

fn build_image_config(
    cli_options: &crate::cli::ImageOptions,
    manifest_processing: Option<&ImageProcessing>,
) -> Result<ImageProcessingConfig, TagCliError> {
    let cli_config = cli_options.to_image_processing_config()?;

    if let Some(p) = manifest_processing
        && let Some(q) = p.quality.filter(|q| *q == 0 || *q > 100)
    {
        return Err(TagCliError::ImageProcessingError(format!(
            "manifest image quality must be between 1 and 100, got {q}"
        )));
    }

    let manifest_config = manifest_processing.map(|p| ImageProcessingConfig {
        no_process: false,
        target_format: p
            .format
            .as_ref()
            .and_then(|f| match f.to_lowercase().as_str() {
                "jpeg" | "jpg" => Some(ImageTargetFormat::Jpeg),
                "png" => Some(ImageTargetFormat::Png),
                _ => None,
            }),
        max_size: p.max_size,
        max_file_size_kb: p.max_file_size,
        quality: p.quality.unwrap_or(90),
        picture_type: None,
    });

    let base = manifest_config.unwrap_or_default();

    Ok(ImageProcessingConfig {
        no_process: cli_config.no_process,
        target_format: cli_config.target_format.or(base.target_format),
        max_size: cli_config.max_size.or(base.max_size),
        max_file_size_kb: cli_config.max_file_size_kb.or(base.max_file_size_kb),
        quality: cli_options.cover_quality.unwrap_or(base.quality),
        picture_type: base.picture_type.clone(),
    })
}
