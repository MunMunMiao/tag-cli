use crate::cli::{InfoArgs, map_format};
use tag_core::error::TagCliError;
use tag_core::workflow::builder::WorkflowBuilder;
use tag_core::workflow::context::Context;
use tag_core::workflow::steps::format_output::FormatOutputStep;
use tag_core::workflow::steps::read_metadata::ReadMetadataStep;

pub fn run(args: &InfoArgs, verbose: bool) -> Result<(), TagCliError> {
    let mut ctx = Context::new(&args.input, false, verbose);
    let workflow = WorkflowBuilder::new()
        .add(Box::new(ReadMetadataStep::new()))
        .add(Box::new(FormatOutputStep::new(map_format(args.format))))
        .build();
    workflow.run(&mut ctx)?;
    println!("{}", ctx.output.unwrap_or_default());
    Ok(())
}
