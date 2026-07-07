use crate::error::TagCliError;
use crate::output::{OutputFormat, format_info};
use crate::workflow::context::Context;
use crate::workflow::step::Step;

#[derive(Debug)]
pub struct FormatOutputStep {
    pub format: OutputFormat,
}

impl FormatOutputStep {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }
}

impl Step for FormatOutputStep {
    fn name(&self) -> &'static str {
        "FormatOutput"
    }

    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        let Some(metadata) = ctx.metadata.as_ref() else {
            return Err(TagCliError::ImageProcessingError(
                "no metadata to format".to_string(),
            ));
        };
        let file = ctx.input_path.to_string_lossy().to_string();
        ctx.output = Some(format_info(metadata, &file, self.format));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taglib::{AudioProperties, Metadata, Picture, Tags};

    fn sample_metadata() -> Metadata {
        Metadata {
            tags: Tags {
                title: Some("Title".to_string()),
                artist: Some("Artist".to_string()),
                album: Some("Album".to_string()),
            },
            properties: Default::default(),
            pictures: vec![Picture {
                mime_type: Some("image/jpeg".to_string()),
                description: None,
                picture_type: Some("Front Cover".to_string()),
                data: vec![0u8; 2048],
            }],
            audio: Some(AudioProperties {
                length_seconds: 60,
                bitrate_kbps: 256,
                sample_rate_hz: 44100,
                channels: 2,
            }),
        }
    }

    #[test]
    fn step_name_and_execute() {
        let step = FormatOutputStep::new(OutputFormat::Table);
        assert_eq!(step.name(), "FormatOutput");

        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.metadata = Some(sample_metadata());
        assert!(step.execute(&mut ctx).is_ok());
        assert!(ctx.output.as_ref().unwrap().contains("File:"));
    }

    #[test]
    fn execute_without_metadata_errors() {
        let step = FormatOutputStep::new(OutputFormat::Json);
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        let err = step.execute(&mut ctx).unwrap_err();
        assert!(err.to_string().contains("no metadata to format"));
    }
}
