use crate::taglib::supported_property_keys;

pub fn format_supported_keys() -> String {
    supported_property_keys().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_supported_keys_includes_known_keys() {
        let out = format_supported_keys();
        assert!(out.contains("TITLE"));
        assert!(out.contains("ARTIST"));
        assert!(out.lines().count() > 0);
    }
}
