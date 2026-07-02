use super::context::Context;
use crate::error::TagCliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Continue,
}

pub trait Step: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &mut Context) -> Result<StepOutcome, TagCliError>;
}
