use crate::cli::{ListKeysArgs, map_format};
use tag_core::output::OutputFormat;
use tag_core::taglib::supported_property_keys;

pub fn run(args: &ListKeysArgs) {
    let keys = supported_property_keys();
    match map_format(args.format) {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(keys).unwrap_or_default());
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(keys).unwrap_or_default());
        }
        OutputFormat::Table => {
            println!("{}", keys.join("\n"));
        }
    }
}
