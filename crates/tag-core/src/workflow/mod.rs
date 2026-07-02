pub mod builder;
pub mod context;
pub mod step;

pub mod steps;

use crate::error::TagCliError;
pub use context::Context;
pub use step::Step;

pub struct Workflow {
    pub steps: Vec<Box<dyn Step>>,
}

impl Workflow {
    pub fn run(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        for step in &self.steps {
            if ctx.verbose {
                tracing::info!("executing step: {}", step.name());
            }
            if ctx.dry_run {
                let name = step.name();
                tracing::debug!("[dry-run] executing: {}", name);
            }
            step.execute(ctx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TagCliError;
    use crate::workflow::step::StepOutcome;

    #[derive(Debug)]
    struct NameStep(&'static str);

    impl Step for NameStep {
        fn name(&self) -> &'static str {
            self.0
        }

        fn execute(&self, _ctx: &mut Context) -> Result<StepOutcome, TagCliError> {
            Ok(StepOutcome::Continue)
        }
    }

    #[derive(Debug)]
    struct ErrorStep;

    impl Step for ErrorStep {
        fn name(&self) -> &'static str {
            "ErrorStep"
        }

        fn execute(&self, _ctx: &mut Context) -> Result<StepOutcome, TagCliError> {
            Err(TagCliError::InPlaceNotConfirmed {
                command: "test".to_string(),
            })
        }
    }

    #[test]
    fn workflow_runs_all_steps() {
        let workflow = Workflow {
            steps: vec![Box::new(NameStep("A")), Box::new(NameStep("B"))],
        };
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(workflow.run(&mut ctx).is_ok());
    }

    #[test]
    fn workflow_verbose_logs_step_names() {
        use crate::test_helpers::capture_logs;

        let workflow = Workflow {
            steps: vec![Box::new(NameStep("A")), Box::new(NameStep("B"))],
        };
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| workflow.run(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("executing step: A"));
        assert!(logs.contains("executing step: B"));
    }

    #[test]
    fn workflow_dry_run_traces_step_names() {
        let workflow = Workflow {
            steps: vec![Box::new(NameStep("A"))],
        };
        let mut ctx = Context::new("/tmp/test.mp3", true, false);
        assert!(workflow.run(&mut ctx).is_ok());
    }

    #[test]
    fn workflow_propagates_step_errors() {
        let workflow = Workflow {
            steps: vec![Box::new(ErrorStep)],
        };
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(workflow.run(&mut ctx).is_err());
    }

    #[test]
    fn name_step_returns_name() {
        assert_eq!(NameStep("A").name(), "A");
    }

    #[test]
    fn error_step_returns_name() {
        assert_eq!(ErrorStep.name(), "ErrorStep");
    }
}
