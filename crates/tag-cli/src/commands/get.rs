use crate::cli::GetArgs;
use tag_core::error::TagCliError;
use tag_core::output::{OutputFormat, format_get};
use tag_core::workflow::builder::WorkflowBuilder;
use tag_core::workflow::context::Context;
use tag_core::workflow::steps::read_metadata::ReadMetadataStep;

pub fn run(args: &GetArgs, verbose: bool) -> Result<(), TagCliError> {
    let mut ctx = Context::new(&args.input, false, verbose);
    ctx.output = None; // FormatGetOutputStep will use metadata and keys

    let workflow = WorkflowBuilder::new()
        .add(Box::new(ReadMetadataStep::new()))
        .add(Box::new(FormatGetOutputStep::new(
            args.keys.iter().map(|k| k.to_ascii_uppercase()).collect(),
            map_format(args.format),
        )))
        .build();
    workflow.run(&mut ctx)?;
    println!("{}", ctx.output.unwrap_or_default());
    Ok(())
}

fn map_format(format: Option<crate::cli::OutputFormat>) -> OutputFormat {
    match format {
        Some(crate::cli::OutputFormat::Json) => OutputFormat::Json,
        Some(crate::cli::OutputFormat::Yaml) => OutputFormat::Yaml,
        _ => OutputFormat::Table,
    }
}

#[derive(Debug)]
struct FormatGetOutputStep {
    keys: Vec<String>,
    format: OutputFormat,
}

impl FormatGetOutputStep {
    fn new(keys: Vec<String>, format: OutputFormat) -> Self {
        Self { keys, format }
    }
}

impl tag_core::workflow::step::Step for FormatGetOutputStep {
    fn name(&self) -> &'static str {
        "FormatGetOutput"
    }

    fn execute(
        &self,
        ctx: &mut Context,
    ) -> Result<tag_core::workflow::step::StepOutcome, TagCliError> {
        let Some(metadata) = ctx.metadata.as_ref() else {
            return Err(TagCliError::ImageProcessingError("no metadata".to_string()));
        };
        ctx.output = Some(format_get(metadata, &self.keys, self.format));
        Ok(tag_core::workflow::step::StepOutcome::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tag_core::workflow::step::Step;
    use taglib_rs::test_utils::generate_mp3;

    #[test]
    fn step_name() {
        let step = FormatGetOutputStep::new(vec![], OutputFormat::Table);
        let _ = step.name();
        let _ = format!("{:?}", step);
    }

    #[test]
    fn execute_without_metadata_errors() {
        let step = FormatGetOutputStep::new(vec!["TITLE".to_string()], OutputFormat::Table);
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        let _ = step.execute(&mut ctx);
    }

    #[test]
    fn map_format_all_variants() {
        assert!(matches!(
            map_format(Some(crate::cli::OutputFormat::Json)),
            OutputFormat::Json
        ));
        assert!(matches!(
            map_format(Some(crate::cli::OutputFormat::Yaml)),
            OutputFormat::Yaml
        ));
        assert!(matches!(map_format(None), OutputFormat::Table));
    }

    #[test]
    fn run_get_success() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);
        let args = GetArgs {
            input,
            keys: vec!["TITLE".to_string()],
            format: None,
        };
        assert!(run(&args, false).is_ok());
    }

    #[test]
    fn run_get_error() {
        let args = GetArgs {
            input: std::path::PathBuf::from("/nonexistent/file.mp3"),
            keys: vec!["TITLE".to_string()],
            format: None,
        };
        assert!(run(&args, false).is_err());
    }
}
