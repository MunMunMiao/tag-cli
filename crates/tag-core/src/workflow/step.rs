use super::context::Context;
use crate::error::TagCliError;

pub trait Step: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError>;
}
