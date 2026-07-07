use crate::error::TagCliError;
use crate::taglib::read_metadata_from_path;
use crate::workflow::context::Context;
use crate::workflow::step::Step;

#[derive(Debug, Default)]
pub struct ReadMetadataStep;

impl ReadMetadataStep {
    pub fn new() -> Self {
        Self
    }
}

impl Step for ReadMetadataStep {
    fn name(&self) -> &'static str {
        "ReadMetadata"
    }

    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        if ctx.verbose {
            tracing::info!("reading metadata from {}", ctx.input_path.display());
        }
        let metadata = read_metadata_from_path(&ctx.input_path)?;
        ctx.metadata = Some(metadata.clone());
        ctx.original_metadata = Some(metadata);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taglib_rs::test_utils::generate_mp3;
    use tempfile::TempDir;

    #[test]
    fn step_name_and_execute() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = ReadMetadataStep::new();
        assert_eq!(step.name(), "ReadMetadata");

        let mut ctx = Context::new(&input, false, false);
        assert!(step.execute(&mut ctx).is_ok());
        assert!(ctx.metadata.is_some());
    }

    #[test]
    fn verbose_logs_reading_metadata() {
        use crate::test_helpers::capture_logs;

        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = ReadMetadataStep::new();
        let mut ctx = Context::new(&input, false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("reading metadata from"));
        assert!(logs.contains(&input.to_string_lossy().to_string()));
    }

    #[test]
    fn execute_with_invalid_path_returns_error() {
        let step = ReadMetadataStep::new();
        let mut ctx = Context::new("\0invalid\0path\0", false, false);
        let err = step.execute(&mut ctx).unwrap_err();
        assert!(matches!(
            err,
            TagCliError::TagLib(crate::taglib::TagError::InvalidPath)
        ));
    }
}
