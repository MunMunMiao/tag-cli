use crate::error::TagCliError;
use crate::image_proc::process_cover_image;
use crate::taglib::Picture;
use crate::workflow::context::{Context, CoverAction, ImageProcessingConfig};
use crate::workflow::step::Step;

#[derive(Debug)]
pub struct ProcessCoverStep {
    pub config: ImageProcessingConfig,
}

impl ProcessCoverStep {
    pub fn new(config: ImageProcessingConfig) -> Self {
        Self { config }
    }
}

impl Step for ProcessCoverStep {
    fn name(&self) -> &'static str {
        "ProcessCover"
    }

    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        let path = match &ctx.cover_action {
            CoverAction::Set(p) => p.clone(),
            _ => return Ok(()),
        };

        let audio_format = ctx
            .audio_format
            .unwrap_or(crate::workflow::context::AudioFormat::Other);
        let processed = process_cover_image(&path, audio_format, &self.config)?;

        if ctx.verbose {
            tracing::info!(
                "processed cover: {} {}x{} -> {} {}x{}",
                processed.original_info.format,
                processed.original_info.width,
                processed.original_info.height,
                processed.processed_info.format,
                processed.processed_info.width,
                processed.processed_info.height,
            );
        }

        ctx.report.messages.push(format!(
            "cover: {} {}x{} {}KB -> {} {}KB",
            processed.original_info.format,
            processed.original_info.width,
            processed.original_info.height,
            processed.original_info.size_bytes / 1024,
            processed.processed_info.format,
            processed.processed_info.size_bytes / 1024,
        ));

        let picture_type = self
            .config
            .picture_type
            .clone()
            .or_else(|| Some("Front Cover".to_string()));

        ctx.processed_cover = Some(Picture {
            mime_type: Some(processed.mime_type),
            description: Some("Cover".to_string()),
            picture_type,
            data: processed.data,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::context::AudioFormat;
    use image::{ImageBuffer, ImageFormat, Rgb};
    use tempfile::TempDir;

    fn write_test_jpg(path: &std::path::Path) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(50, 50, Rgb([255, 0, 0]));
        let mut file = std::fs::File::create(path).unwrap();
        img.write_to(&mut file, ImageFormat::Jpeg).unwrap();
    }

    #[test]
    fn step_name() {
        let step = ProcessCoverStep::new(ImageProcessingConfig::default());
        assert_eq!(step.name(), "ProcessCover");
    }

    #[test]
    fn keep_action_returns_continue() {
        let step = ProcessCoverStep::new(ImageProcessingConfig::default());
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.cover_action = CoverAction::Keep;
        assert!(step.execute(&mut ctx).is_ok());
        assert!(ctx.processed_cover.is_none());
    }

    #[test]
    fn clear_action_returns_continue() {
        let step = ProcessCoverStep::new(ImageProcessingConfig::default());
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.cover_action = CoverAction::Clear;
        assert!(step.execute(&mut ctx).is_ok());
        assert!(ctx.processed_cover.is_none());
    }

    #[test]
    fn set_action_processes_cover() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        write_test_jpg(&cover);

        let step = ProcessCoverStep::new(ImageProcessingConfig::default());
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.audio_format = Some(AudioFormat::Mpeg);
        ctx.cover_action = CoverAction::Set(cover);

        assert!(step.execute(&mut ctx).is_ok());
        assert!(ctx.processed_cover.is_some());
        assert!(!ctx.report.messages.is_empty());
    }

    #[test]
    fn verbose_logs_processed_cover() {
        use crate::test_helpers::capture_logs;

        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        write_test_jpg(&cover);

        let step = ProcessCoverStep::new(ImageProcessingConfig::default());
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        ctx.audio_format = Some(AudioFormat::Mpeg);
        ctx.cover_action = CoverAction::Set(cover);

        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("processed cover:"));
        assert!(logs.contains("50x50"));
    }

    #[test]
    fn default_picture_type_is_front_cover() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        write_test_jpg(&cover);

        let step = ProcessCoverStep::new(ImageProcessingConfig::default());
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.audio_format = Some(AudioFormat::Mpeg);
        ctx.cover_action = CoverAction::Set(cover);

        step.execute(&mut ctx).unwrap();
        assert_eq!(
            ctx.processed_cover.unwrap().picture_type,
            Some("Front Cover".to_string())
        );
    }

    #[test]
    fn custom_picture_type_is_used() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        write_test_jpg(&cover);

        let config = ImageProcessingConfig {
            picture_type: Some("Back Cover".to_string()),
            ..Default::default()
        };
        let step = ProcessCoverStep::new(config);
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        ctx.audio_format = Some(AudioFormat::Mpeg);
        ctx.cover_action = CoverAction::Set(cover);

        step.execute(&mut ctx).unwrap();
        assert_eq!(
            ctx.processed_cover.unwrap().picture_type,
            Some("Back Cover".to_string())
        );
    }
}
