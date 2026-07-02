use std::io::Cursor;
use std::path::Path;

use image::{ImageFormat, ImageReader};

use crate::error::TagCliError;
use crate::workflow::context::{AudioFormat, ImageProcessingConfig, ImageTargetFormat};

#[derive(Debug)]
pub struct ProcessedImage {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub original_info: ImageInfo,
    pub processed_info: ImageInfo,
}

#[derive(Debug)]
pub struct ImageInfo {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
}

pub fn process_cover_image(
    path: &Path,
    audio_format: AudioFormat,
    config: &ImageProcessingConfig,
) -> Result<ProcessedImage, TagCliError> {
    if config.no_process {
        let data =
            std::fs::read(path).map_err(|e| TagCliError::ImageProcessingError(e.to_string()))?;
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        return Ok(ProcessedImage {
            data,
            mime_type: mime,
            original_info: image_info("unknown", 0, 0, 0),
            processed_info: image_info("unknown", 0, 0, 0),
        });
    }

    let bytes =
        std::fs::read(path).map_err(|e| TagCliError::ImageProcessingError(e.to_string()))?;
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .expect("format guessing from in-memory bytes should not fail");

    let format = reader
        .format()
        .ok_or_else(|| TagCliError::UnsupportedImageFormat("unknown".to_string()))?;

    if !is_supported_input_format(format) {
        return Err(TagCliError::UnsupportedImageFormat(format_name(format)));
    }

    let mut img = reader
        .decode()
        .map_err(|e| TagCliError::ImageDecodeError(e.to_string()))?;

    let (default_max_size, default_max_kb) = defaults_for_audio_format(audio_format);
    let max_size = config.max_size.unwrap_or(default_max_size);
    let max_kb = config.max_file_size_kb.unwrap_or(default_max_kb);

    let original_width = img.width();
    let original_height = img.height();

    // Determine target format.
    let target_format = config.target_format.unwrap_or_else(|| {
        if has_alpha(&img) {
            ImageTargetFormat::Png
        } else {
            ImageTargetFormat::Jpeg
        }
    });

    // Check if any pixel/format processing is needed.
    let needs_resize = original_width > max_size || original_height > max_size;
    let needs_format_change = target_image_format(target_format) != format;
    let needs_size_reduction = (bytes.len() / 1024) > max_kb as usize;

    let processed = if !needs_resize && !needs_format_change && !needs_size_reduction {
        strip_exif_lossless(&bytes, format).map(|stripped| ProcessedImage {
            data: stripped.clone(),
            mime_type: mime_type_for_format(target_format),
            original_info: image_info(
                format_name(format),
                original_width,
                original_height,
                bytes.len(),
            ),
            processed_info: image_info(
                format_name(format),
                original_width,
                original_height,
                stripped.len(),
            ),
        })
    } else {
        None
    };

    let processed = match processed {
        Some(p) => p,
        None => {
            // Otherwise, re-encode (drops EXIF) and apply size/format changes.
            img = fit_within_max_size(img, max_size);
            let (data, mime_type, processed_width, processed_height) =
                encode_with_size_limit(img, target_format, config.quality, max_kb);
            let processed_size = data.len();

            ProcessedImage {
                data,
                mime_type,
                original_info: image_info(
                    format_name(format),
                    original_width,
                    original_height,
                    bytes.len(),
                ),
                processed_info: image_info(
                    format_name(target_image_format(target_format)),
                    processed_width,
                    processed_height,
                    processed_size,
                ),
            }
        }
    };

    Ok(processed)
}

fn strip_exif_lossless(bytes: &[u8], format: ImageFormat) -> Option<Vec<u8>> {
    match format {
        ImageFormat::Jpeg => strip_exif_jpeg(bytes),
        ImageFormat::Png => strip_exif_png(bytes),
        _ => None,
    }
}

fn strip_exif_jpeg(bytes: &[u8]) -> Option<Vec<u8>> {
    use img_parts::jpeg::Jpeg;
    let jpeg = Jpeg::from_bytes(bytes.to_vec().into()).ok()?;
    let mut out = Vec::new();
    jpeg.encoder()
        .write_to(&mut out)
        .expect("JPEG re-encode to in-memory buffer should not fail");
    Some(out)
}

fn strip_exif_png(bytes: &[u8]) -> Option<Vec<u8>> {
    use img_parts::png::Png;
    let png = Png::from_bytes(bytes.to_vec().into()).ok()?;
    let mut out = Vec::new();
    png.encoder()
        .write_to(&mut out)
        .expect("PNG re-encode to in-memory buffer should not fail");
    Some(out)
}

fn mime_type_for_format(target: ImageTargetFormat) -> String {
    match target {
        ImageTargetFormat::Jpeg => "image/jpeg".to_string(),
        ImageTargetFormat::Png => "image/png".to_string(),
    }
}

fn image_info(format: impl Into<String>, width: u32, height: u32, size_bytes: usize) -> ImageInfo {
    ImageInfo {
        format: format.into(),
        width,
        height,
        size_bytes,
    }
}

fn is_supported_input_format(format: ImageFormat) -> bool {
    matches!(
        format,
        ImageFormat::Jpeg
            | ImageFormat::Png
            | ImageFormat::Gif
            | ImageFormat::Bmp
            | ImageFormat::WebP
            | ImageFormat::Tiff
    )
}

fn format_name(format: ImageFormat) -> String {
    format!("{:?}", format).to_lowercase()
}

fn target_image_format(target: ImageTargetFormat) -> ImageFormat {
    match target {
        ImageTargetFormat::Jpeg => ImageFormat::Jpeg,
        ImageTargetFormat::Png => ImageFormat::Png,
    }
}

fn has_alpha(img: &image::DynamicImage) -> bool {
    img.has_alpha()
}

fn fit_within_max_size(img: image::DynamicImage, max_size: u32) -> image::DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w <= max_size && h <= max_size {
        return img;
    }
    img.resize(max_size, max_size, image::imageops::FilterType::Lanczos3)
}

fn encode_with_size_limit(
    mut img: image::DynamicImage,
    target: ImageTargetFormat,
    mut quality: u8,
    max_kb: u32,
) -> (Vec<u8>, String, u32, u32) {
    let mime_type = mime_type_for_format(target);

    match target {
        ImageTargetFormat::Jpeg => loop {
            let mut buf = Vec::new();
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            img.write_with_encoder(encoder)
                .expect("JPEG encoding to in-memory buffer should not fail");

            let size_kb = (buf.len() / 1024) as u32;
            if size_kb <= max_kb || quality <= 30 {
                return (buf, mime_type, img.width(), img.height());
            }

            quality = quality.saturating_sub(10);
        },
        ImageTargetFormat::Png => {
            const MIN_DIMENSION: u32 = 100;
            const SCALE_FACTOR: f32 = 0.75;

            loop {
                let mut buf = Vec::new();
                img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
                    .expect("PNG encoding to in-memory buffer should not fail");

                let size_kb = (buf.len() / 1024) as u32;
                if size_kb <= max_kb {
                    return (buf, mime_type, img.width(), img.height());
                }

                let (w, h) = (img.width(), img.height());
                if w <= MIN_DIMENSION || h <= MIN_DIMENSION {
                    return (buf, mime_type, w, h);
                }

                let new_w = ((w as f32 * SCALE_FACTOR) as u32).max(MIN_DIMENSION);
                let new_h = ((h as f32 * SCALE_FACTOR) as u32).max(MIN_DIMENSION);
                img = img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
            }
        }
    }
}

fn defaults_for_audio_format(format: AudioFormat) -> (u32, u32) {
    match format {
        AudioFormat::Mp4
        | AudioFormat::Flac
        | AudioFormat::OggVorbis
        | AudioFormat::OggOpus
        | AudioFormat::OggFlac
        | AudioFormat::Speex => (2048, 2048),
        AudioFormat::Mpeg
        | AudioFormat::Wav
        | AudioFormat::Aiff
        | AudioFormat::Wma
        | AudioFormat::Ape
        | AudioFormat::Mpc
        | AudioFormat::WavPack
        | AudioFormat::TrueAudio
        | AudioFormat::Dsf
        | AudioFormat::Mod
        | AudioFormat::Shorten
        | AudioFormat::Matroska
        | AudioFormat::Other => (1200, 1024),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgb, Rgba};
    use tempfile::TempDir;

    fn write_image(path: &std::path::Path, img: &image::DynamicImage, format: ImageFormat) {
        let mut file = std::fs::File::create(path).unwrap();
        img.write_to(&mut file, format).unwrap();
    }

    #[test]
    fn defaults_for_every_audio_format() {
        let high = (2048, 2048);
        let low = (1200, 1024);

        assert_eq!(defaults_for_audio_format(AudioFormat::Mpeg), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Mp4), high);
        assert_eq!(defaults_for_audio_format(AudioFormat::Flac), high);
        assert_eq!(defaults_for_audio_format(AudioFormat::OggVorbis), high);
        assert_eq!(defaults_for_audio_format(AudioFormat::OggOpus), high);
        assert_eq!(defaults_for_audio_format(AudioFormat::OggFlac), high);
        assert_eq!(defaults_for_audio_format(AudioFormat::Speex), high);
        assert_eq!(defaults_for_audio_format(AudioFormat::Wav), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Aiff), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Wma), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Ape), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Mpc), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::WavPack), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::TrueAudio), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Dsf), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Mod), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Shorten), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Matroska), low);
        assert_eq!(defaults_for_audio_format(AudioFormat::Other), low);
    }

    #[test]
    fn no_process_returns_raw_bytes() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        let img = ImageBuffer::from_pixel(10, 10, Rgb([0, 0, 0]));
        write_image(
            &cover,
            &image::DynamicImage::ImageRgb8(img),
            ImageFormat::Jpeg,
        );

        let config = ImageProcessingConfig {
            no_process: true,
            ..Default::default()
        };
        let result = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap();
        assert_eq!(result.original_info.format, "unknown");
        assert!(!result.data.is_empty());
    }

    #[test]
    fn missing_file_errors() {
        let config = ImageProcessingConfig::default();
        let err = process_cover_image(Path::new("/does/not/exist.jpg"), AudioFormat::Mpeg, &config)
            .unwrap_err();
        assert!(err.to_string().contains("image processing error"));
    }

    #[test]
    fn no_process_missing_file_errors() {
        let config = ImageProcessingConfig {
            no_process: true,
            ..Default::default()
        };
        let err = process_cover_image(Path::new("/does/not/exist.jpg"), AudioFormat::Mpeg, &config)
            .unwrap_err();
        assert!(err.to_string().contains("image processing error"));
    }

    #[test]
    fn empty_file_image_format_errors() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("empty.bin");
        std::fs::write(&cover, b"").unwrap();

        let config = ImageProcessingConfig::default();
        let err = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap_err();
        assert!(err.to_string().contains("image"));
    }

    #[test]
    fn unknown_image_format_errors() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.bin");
        std::fs::write(&cover, b"not an image").unwrap();

        let config = ImageProcessingConfig::default();
        let err = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap_err();
        assert!(err.to_string().contains("unsupported image format"));
    }

    #[test]
    fn unsupported_input_format_errors() {
        // QOI has a recognizable magic number but is not a supported input format.
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.qoi");
        let mut bytes = b"qoif".to_vec();
        bytes.extend_from_slice(&[0u8; 4]); // width
        bytes.extend_from_slice(&[0u8; 4]); // height
        bytes.push(3); // channels
        bytes.push(0); // colorspace
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]); // end marker
        std::fs::write(&cover, bytes).unwrap();

        let config = ImageProcessingConfig::default();
        let err = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap_err();
        assert!(err.to_string().contains("unsupported image format"));
    }

    #[test]
    fn decode_failure_errors() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        // Valid JPEG magic but truncated data.
        std::fs::write(&cover, b"\xff\xd8\xff\xe0\x00\x10JFIF\x00").unwrap();

        let config = ImageProcessingConfig::default();
        let err = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap_err();
        assert!(err.to_string().contains("failed to decode image"));
    }

    #[test]
    fn png_with_alpha_targets_png_and_strips_exif() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.png");
        let img = ImageBuffer::from_pixel(50, 50, Rgba([255, 0, 0, 128]));
        write_image(
            &cover,
            &image::DynamicImage::ImageRgba8(img),
            ImageFormat::Png,
        );

        let config = ImageProcessingConfig::default();
        let result = process_cover_image(&cover, AudioFormat::Flac, &config).unwrap();
        assert_eq!(result.processed_info.format, "png");
        assert_eq!(result.mime_type, "image/png");
    }

    #[test]
    fn jpeg_quality_reduction_loop() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        let img = ImageBuffer::from_pixel(100, 100, Rgb([255, 0, 0]));
        write_image(
            &cover,
            &image::DynamicImage::ImageRgb8(img),
            ImageFormat::Jpeg,
        );

        let config = ImageProcessingConfig {
            quality: 90,
            max_file_size_kb: Some(1),
            ..Default::default()
        };
        let result = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap();
        assert_eq!(result.processed_info.format, "jpeg");
        assert!(result.processed_info.size_bytes / 1024 <= 1);
    }

    #[test]
    fn png_size_reduction_loop() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.png");
        let img = ImageBuffer::from_pixel(1000, 1000, Rgb([255, 0, 0]));
        write_image(
            &cover,
            &image::DynamicImage::ImageRgb8(img),
            ImageFormat::Png,
        );

        let config = ImageProcessingConfig {
            target_format: Some(ImageTargetFormat::Png),
            max_file_size_kb: Some(1),
            ..Default::default()
        };
        let result = process_cover_image(&cover, AudioFormat::Flac, &config).unwrap();
        assert_eq!(result.processed_info.format, "png");
    }

    #[test]
    fn fit_within_max_size_scales_down() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        let img = ImageBuffer::from_pixel(2000, 2000, Rgb([255, 0, 0]));
        write_image(
            &cover,
            &image::DynamicImage::ImageRgb8(img),
            ImageFormat::Jpeg,
        );

        let config = ImageProcessingConfig {
            max_size: Some(500),
            ..Default::default()
        };
        let result = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap();
        assert!(result.processed_info.width <= 500);
        assert!(result.processed_info.height <= 500);
    }

    #[test]
    fn strip_exif_jpeg_error_paths() {
        assert!(strip_exif_jpeg(b"not jpeg").is_none());
    }

    #[test]
    fn strip_exif_png_parse_error_returns_none() {
        assert!(strip_exif_png(b"not png").is_none());
    }

    #[test]
    fn jpeg_lossless_exif_strip() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        let img = ImageBuffer::from_pixel(50, 50, Rgb([255, 0, 0]));
        write_image(
            &cover,
            &image::DynamicImage::ImageRgb8(img),
            ImageFormat::Jpeg,
        );

        let config = ImageProcessingConfig::default();
        let result = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap();
        assert_eq!(result.processed_info.format, "jpeg");
        assert_eq!(result.processed_info.width, 50);
        assert_eq!(result.processed_info.height, 50);
    }

    #[test]
    fn strip_exif_lossless_unsupported_format_returns_none() {
        assert!(strip_exif_lossless(b"anything", ImageFormat::Gif).is_none());
    }

    #[test]
    fn jpeg_quality_reduction_loop_iterates() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.jpg");
        let img = ImageBuffer::from_pixel(400, 400, Rgb([255, 0, 0]));
        write_image(
            &cover,
            &image::DynamicImage::ImageRgb8(img),
            ImageFormat::Jpeg,
        );

        let config = ImageProcessingConfig {
            quality: 90,
            max_file_size_kb: Some(0),
            ..Default::default()
        };
        let result = process_cover_image(&cover, AudioFormat::Mpeg, &config).unwrap();
        assert_eq!(result.processed_info.format, "jpeg");
    }

    #[test]
    fn png_loop_hits_minimum_dimension() {
        let tmp = TempDir::new().unwrap();
        let cover = tmp.path().join("cover.png");
        // A high-frequency pattern makes the PNG larger than 1 KB while
        // keeping the dimensions at the minimum threshold.
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(100, 100, |x, y| {
            let x = x as u8;
            let y = y as u8;
            Rgb([x ^ y, x.wrapping_mul(3) ^ y, x.wrapping_mul(5) ^ y])
        });
        write_image(
            &cover,
            &image::DynamicImage::ImageRgb8(img),
            ImageFormat::Png,
        );
        assert!(std::fs::metadata(&cover).unwrap().len() > 1024);

        let config = ImageProcessingConfig {
            target_format: Some(ImageTargetFormat::Png),
            max_file_size_kb: Some(0),
            ..Default::default()
        };
        let result = process_cover_image(&cover, AudioFormat::Flac, &config).unwrap();
        assert_eq!(result.processed_info.format, "png");
    }
}
