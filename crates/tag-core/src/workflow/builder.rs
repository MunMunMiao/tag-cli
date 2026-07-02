use super::{Workflow, step::Step};

pub struct WorkflowBuilder {
    steps: Vec<Box<dyn Step>>,
}

impl Default for WorkflowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowBuilder {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, step: Box<dyn Step>) -> Self {
        self.steps.push(step);
        self
    }

    pub fn build(self) -> Workflow {
        Workflow { steps: self.steps }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TagCliError;
    use crate::workflow::context::Context;
    use crate::workflow::step::Step;

    #[derive(Debug)]
    struct DummyStep;

    impl Step for DummyStep {
        fn name(&self) -> &'static str {
            "Dummy"
        }

        fn execute(
            &self,
            _ctx: &mut Context,
        ) -> Result<crate::workflow::step::StepOutcome, TagCliError> {
            Ok(crate::workflow::step::StepOutcome::Continue)
        }
    }

    #[test]
    fn default_builder_matches_new() {
        let default: WorkflowBuilder = WorkflowBuilder::default();
        let explicit = WorkflowBuilder::new();
        assert_eq!(default.build().steps.len(), explicit.build().steps.len());
    }

    #[test]
    fn builder_adds_steps() {
        let workflow = WorkflowBuilder::new().add(Box::new(DummyStep)).build();
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.steps[0].name(), "Dummy");

        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        workflow.run(&mut ctx).unwrap();
    }
}
