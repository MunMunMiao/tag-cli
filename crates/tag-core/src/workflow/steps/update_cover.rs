use crate::error::TagCliError;
use crate::workflow::context::{Context, CoverAction};
use crate::workflow::step::Step;

#[derive(Debug)]
pub struct UpdateCoverStep;

impl Default for UpdateCoverStep {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateCoverStep {
    pub fn new() -> Self {
        Self
    }
}

impl Step for UpdateCoverStep {
    fn name(&self) -> &'static str {
        "UpdateCover"
    }

    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        // CoverAction 已在 SaveFileStep 中消费；此处仅做校验或记录。
        if matches!(ctx.cover_action, CoverAction::Set(_)) && ctx.processed_cover.is_none() {
            return Err(TagCliError::ImageProcessingError(
                "cover was requested but not processed".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_name_and_keep_continue() {
        let step = UpdateCoverStep::new();
        assert_eq!(step.name(), "UpdateCover");

        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.cover_action = CoverAction::Keep;
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn set_without_processed_cover_errors() {
        let step = UpdateCoverStep::new();
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.cover_action = CoverAction::Set(std::path::PathBuf::from("/tmp/cover.jpg"));
        let err = step.execute(&mut ctx).unwrap_err();
        assert!(
            err.to_string()
                .contains("cover was requested but not processed")
        );
    }

    #[test]
    #[allow(clippy::default_constructed_unit_structs)]
    fn default_step_matches_new() {
        let default = UpdateCoverStep::default();
        let explicit = UpdateCoverStep::new();
        assert_eq!(default.name(), explicit.name());
    }
}
