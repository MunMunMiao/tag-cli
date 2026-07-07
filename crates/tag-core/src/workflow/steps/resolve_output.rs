use std::path::PathBuf;

use crate::error::TagCliError;
use crate::workflow::context::Context;
use crate::workflow::step::Step;

#[derive(Debug)]
pub struct ResolveOutputPathStep {
    pub explicit_output: Option<PathBuf>,
    pub require_confirm: bool,
    pub command_name: String,
}

impl ResolveOutputPathStep {
    pub fn new(
        explicit_output: Option<PathBuf>,
        require_confirm: bool,
        command_name: impl Into<String>,
    ) -> Self {
        Self {
            explicit_output,
            require_confirm,
            command_name: command_name.into(),
        }
    }
}

impl Step for ResolveOutputPathStep {
    fn name(&self) -> &'static str {
        "ResolveOutputPath"
    }

    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        let output = match &self.explicit_output {
            Some(o) => {
                if o == &ctx.input_path {
                    return Err(TagCliError::SameInputOutput);
                }
                o.clone()
            }
            None => {
                if self.require_confirm && !ctx.dry_run {
                    return Err(TagCliError::InPlaceNotConfirmed {
                        command: self.command_name.clone(),
                    });
                }
                ctx.input_path.clone()
            }
        };
        if ctx.verbose {
            tracing::info!("output path resolved to {}", output.display());
        }
        ctx.output_path = Some(output);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn step_name_and_in_place_without_confirm() {
        let step = ResolveOutputPathStep::new(None, false, "tag-cli test");
        assert_eq!(step.name(), "ResolveOutputPath");

        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(step.execute(&mut ctx).is_ok());
        assert_eq!(ctx.output_path, Some(PathBuf::from("/tmp/test.mp3")));
    }

    #[test]
    fn verbose_logs_resolved_output_path() {
        use crate::test_helpers::capture_logs;

        let step =
            ResolveOutputPathStep::new(Some(PathBuf::from("/tmp/out.mp3")), false, "tag-cli test");
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("output path resolved to"));
        assert!(logs.contains("/tmp/out.mp3"));
    }

    #[test]
    fn in_place_requires_confirmation() {
        let step = ResolveOutputPathStep::new(None, true, "tag-cli test");
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        let err = step.execute(&mut ctx).unwrap_err();
        assert_eq!(
            err.to_string(),
            "tag-cli test: in-place modification requires -y/--yes (use --dry-run to preview)"
        );
    }

    #[test]
    fn dry_run_skips_confirmation() {
        let step = ResolveOutputPathStep::new(None, true, "tag-cli test");
        let mut ctx = Context::new("/tmp/test.mp3", true, false);
        assert!(step.execute(&mut ctx).is_ok());
        assert_eq!(ctx.output_path, Some(PathBuf::from("/tmp/test.mp3")));
    }

    #[test]
    fn explicit_output_is_used() {
        let step =
            ResolveOutputPathStep::new(Some(PathBuf::from("/tmp/out.mp3")), false, "tag-cli test");
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(step.execute(&mut ctx).is_ok());
        assert_eq!(ctx.output_path, Some(PathBuf::from("/tmp/out.mp3")));
    }

    #[test]
    fn same_input_output_rejected() {
        let path = PathBuf::from("/tmp/test.mp3");
        let step = ResolveOutputPathStep::new(Some(path.clone()), false, "tag-cli test");
        let mut ctx = Context::new(&path, false, false);
        let err = step.execute(&mut ctx).unwrap_err();
        assert_eq!(
            err.to_string(),
            "output path cannot be the same as input path"
        );
    }
}
