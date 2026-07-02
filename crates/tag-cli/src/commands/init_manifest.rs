use crate::cli::{InitManifestArgs, TemplateName};
use tag_core::config::Manifest;
use tag_core::error::TagCliError;

const DEFAULT_TEMPLATE: &str = r#"defaults:
  ARTIST: "Example Artist"
  ALBUM: "Example Album"
  DATE: "2026"
files:
  - path: "01-intro.mp3"
    tags:
      TITLE: "Intro"
      TRACKNUMBER: "1"
    cover: "artwork.jpg"
"#;

const TEMPLATES: &[(TemplateName, &str)] = &[
    (
        TemplateName::Classical,
        include_str!("../../templates/classical.yaml"),
    ),
    (
        TemplateName::Podcast,
        include_str!("../../templates/podcast.yaml"),
    ),
    (
        TemplateName::Radio,
        include_str!("../../templates/radio.yaml"),
    ),
    (
        TemplateName::Education,
        include_str!("../../templates/education.yaml"),
    ),
    (
        TemplateName::Vinyl,
        include_str!("../../templates/vinyl.yaml"),
    ),
    (
        TemplateName::Release,
        include_str!("../../templates/release.yaml"),
    ),
];

pub fn run(args: &InitManifestArgs) -> Result<(), TagCliError> {
    if !crate::cli::Cli::is_confirmed(args.yes) {
        return Err(TagCliError::WriteNotConfirmed {
            command: "tag-cli init-manifest".to_string(),
        });
    }

    let content = match args.template {
        Some(name) => {
            let raw = TEMPLATES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, s)| *s)
                .unwrap_or(DEFAULT_TEMPLATE);
            // Validate the selected template is a well-formed manifest.
            let _: Manifest = serde_yaml::from_str(raw).map_err(|e| {
                TagCliError::ManifestRead(format!("built-in template {:?} is invalid: {e}", name))
            })?;
            raw
        }
        None => DEFAULT_TEMPLATE,
    };

    std::fs::write(&args.output, content).map_err(TagCliError::Io)?;
    crate::report::status(format!(
        "manifest template written to {}",
        args.output.display()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_templates_are_valid_manifests() {
        for (name, content) in TEMPLATES {
            let result: Result<Manifest, _> = serde_yaml::from_str(content);
            assert!(
                result.is_ok(),
                "template {:?} should be valid YAML manifest: {:?}",
                name,
                result.err()
            );
        }
    }
}
