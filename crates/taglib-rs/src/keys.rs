use std::collections::HashSet;
use std::sync::OnceLock;

const PROPERTY_MAPPING: &str = include_str!("../vendor/taglib/taglib/toolkit/propertymapping.dox");

fn parse_property_mapping_keys() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for line in PROPERTY_MAPPING.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }

        let mut parts = line.split('|').map(|s| s.trim());
        let _leading = parts.next();
        let first_cell = parts.next().unwrap_or("");

        if first_cell.is_empty() || first_cell.eq_ignore_ascii_case("key") {
            continue;
        }
        if first_cell.starts_with('-') {
            continue;
        }

        out.push(first_cell.to_string());
    }

    let mut seen = HashSet::<String>::new();
    out.retain(|k| seen.insert(k.clone()));
    out
}

pub fn supported_property_keys() -> &'static [String] {
    static KEYS: OnceLock<Vec<String>> = OnceLock::new();
    KEYS.get_or_init(parse_property_mapping_keys)
}

pub fn supported_property_keys_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| supported_property_keys().iter().cloned().collect())
}
