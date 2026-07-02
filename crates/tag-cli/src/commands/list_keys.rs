use crate::cli::ListKeysArgs;
use tag_core::error::TagCliError;
use tag_core::output::OutputFormat;
use tag_core::taglib::supported_property_keys;

pub fn run(args: &ListKeysArgs) -> Result<(), TagCliError> {
    let keys = supported_property_keys();
    match map_format(args.format) {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(keys).unwrap_or_default());
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(keys).unwrap_or_default());
        }
        OutputFormat::Table => {
            for key in keys.iter() {
                println!("{}", key);
            }
        }
    }
    Ok(())
}

fn map_format(format: Option<crate::cli::OutputFormat>) -> OutputFormat {
    match format {
        Some(crate::cli::OutputFormat::Json) => OutputFormat::Json,
        Some(crate::cli::OutputFormat::Yaml) => OutputFormat::Yaml,
        _ => OutputFormat::Table,
    }
}
