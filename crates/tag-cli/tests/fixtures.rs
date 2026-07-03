use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use image::{ImageBuffer, ImageFormat, Rgb};

static FIXTURE_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Returns the path to a generated audio fixture.
///
/// Audio fixtures are generated on first access into a shared temporary
/// directory that lives for the lifetime of the test process.
pub fn audio_fixture(name: &str) -> PathBuf {
    fixture_dir().path().join("audio").join(name)
}

/// Returns the path to a generated image fixture.
///
/// Image fixtures are generated on first access into a shared temporary
/// directory that lives for the lifetime of the test process.
#[allow(dead_code)]
pub fn image_fixture(name: &str) -> PathBuf {
    fixture_dir().path().join("images").join(name)
}

fn fixture_dir() -> &'static tempfile::TempDir {
    FIXTURE_DIR.get_or_init(|| {
        let dir = tempfile::TempDir::new().expect("failed to create temp dir for fixtures");
        generate_all_fixtures(dir.path());
        dir
    })
}

fn generate_all_fixtures(base: &Path) {
    verify_ffmpeg_available();

    fs::create_dir_all(base.join("audio")).expect("create audio fixture dir");
    fs::create_dir_all(base.join("images")).expect("create images fixture dir");

    generate_audio_fixtures(base);
    generate_image_fixtures(base);
}

fn verify_ffmpeg_available() {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "ffmpeg is required for integration tests but was not found on PATH: {}. \
                 Please install ffmpeg (e.g. `sudo apt-get install ffmpeg` or `brew install ffmpeg`).",
                e
            )
        });
}

fn generate_audio_fixtures(base: &Path) {
    // Core formats required for the existing test suite.
    let spec: Vec<(&str, &[&str])> = vec![
        ("sample_flac.flac", &[]),
        ("sample_mp3.mp3", &["-c:a", "libmp3lame", "-q:a", "4"]),
        ("sample_mp2.mp2", &["-c:a", "mp2", "-b:a", "192k"]),
        ("sample_m4a.m4a", &["-c:a", "aac", "-b:a", "128k"]),
        (
            "sample_ogg.ogg",
            &["-c:a", "vorbis", "-strict", "experimental", "-q:a", "3"],
        ),
        (
            "sample_opus.opus",
            &["-ar", "48000", "-c:a", "libopus", "-b:a", "128k"],
        ),
        ("sample_wav.wav", &["-c:a", "pcm_s16le"]),
        ("sample_aiff.aiff", &["-c:a", "pcm_s16be"]),
    ];

    for (name, codec_args) in spec {
        let output = base.join("audio").join(name);
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("sine=frequency=1000:duration=1")
            .arg("-ar")
            .arg("44100")
            .arg("-ac")
            .arg("2")
            .arg("-metadata")
            .arg("TITLE=Original Title");
        cmd.args(codec_args);
        cmd.arg(&output);
        run_ffmpeg_or_panic(cmd, &format!("audio/{name}"), &output);
    }

    // Optional formats that depend on ffmpeg build-time encoders.
    // These are generated when the encoder is available and skipped with a
    // warning otherwise, so tests do not fail on missing encoders.
    let optional: Vec<(&str, &[&str])> = vec![
        (
            "sample_oga.oga",
            &[
                "-c:a",
                "flac",
                "-f",
                "ogg",
                "-metadata",
                "TITLE=Original Title",
            ],
        ),
        (
            "sample_spx.spx",
            &[
                "-ar",
                "16000",
                "-ac",
                "1",
                "-c:a",
                "libspeex",
                "-metadata",
                "TITLE=Original Title",
            ],
        ),
        (
            "sample_wma.wma",
            &["-c:a", "wmav2", "-metadata", "TITLE=Original Title"],
        ),
        (
            "sample_wv.wv",
            &["-c:a", "wavpack", "-metadata", "TITLE=Original Title"],
        ),
        (
            "sample_mka.mka",
            &["-c:a", "flac", "-metadata", "TITLE=Original Title"],
        ),
    ];

    for (name, codec_args) in optional {
        let output = base.join("audio").join(name);
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("sine=frequency=1000:duration=1")
            .arg("-ar")
            .arg("44100")
            .arg("-ac")
            .arg("2");
        cmd.args(codec_args);
        cmd.arg(&output);
        if let Err(e) = run_ffmpeg(cmd, &format!("audio/{name}"), &output) {
            eprintln!("Warning: skipping optional fixture {name}: {e}");
        }
    }
}

fn generate_image_fixtures(base: &Path) {
    generate_image_from_color(base, "cover_jpg.jpg", "red", 100, 100, &["-q:v", "10"]);
    generate_image_from_color(base, "cover_png.png", "red", 100, 100, &[]);
    generate_image_from_color(base, "cover_gif.gif", "red", 100, 100, &[]);
    generate_image_from_color(base, "cover_bmp.bmp", "red", 100, 100, &[]);
    // ffmpeg-generated TIFF files may use a color type that the Rust `image`
    // crate cannot decode, so generate the TIFF fixture directly with `image`.
    generate_tiff_via_image(&base.join("images").join("cover_tiff.tiff"));

    // WebP: try ffmpeg's libwebp encoder first. If it is unavailable, fall back
    // to a minimal valid WebP generated with the `image` crate.
    let webp_path = base.join("images").join("cover_webp.webp");
    let mut webp_cmd = Command::new("ffmpeg");
    webp_cmd
        .arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("color=c=red:s=100x100:d=1")
        .arg("-frames:v")
        .arg("1")
        .arg("-c:v")
        .arg("libwebp")
        .arg("-q:v")
        .arg("80")
        .arg(&webp_path);

    if run_ffmpeg(webp_cmd, "images/cover_webp.webp", &webp_path).is_err() {
        if has_encoder("libwebp") {
            panic!(
                "ffmpeg has the libwebp encoder but failed to generate cover_webp.webp; \
                 see stderr above"
            );
        }
        generate_webp_via_image(&webp_path);
    }

    // Large JPEG for resize tests.
    generate_image_from_color(base, "cover_large.jpg", "blue", 2500, 2500, &["-q:v", "30"]);
}

fn generate_image_from_color(
    base: &Path,
    name: &str,
    color: &str,
    width: u32,
    height: u32,
    extra_args: &[&str],
) {
    let output = base.join("images").join(name);
    let source = format!("color=c={color}:s={width}x{height}:d=1");

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg(source)
        .arg("-frames:v")
        .arg("1");
    cmd.args(extra_args);
    cmd.arg(&output);

    run_ffmpeg_or_panic(cmd, &format!("images/{name}"), &output);
}

fn generate_webp_via_image(path: &Path) {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(100, 100, Rgb([255, 0, 0]));
    let mut file = File::create(path).expect("create webp fallback file");
    img.write_to(&mut file, ImageFormat::WebP)
        .expect("write webp fallback");
}

fn generate_tiff_via_image(path: &Path) {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(100, 100, Rgb([255, 0, 0]));
    let mut file = File::create(path).expect("create tiff file");
    img.write_to(&mut file, ImageFormat::Tiff)
        .expect("write tiff file");
}

fn has_encoder(encoder: &str) -> bool {
    let output = match Command::new("ffmpeg").arg("-encoders").output() {
        Ok(out) => out,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().any(|line| {
        // ffmpeg -encoders lines look like:
        //   V..... libwebp          WebP image
        line.split_whitespace().nth(1) == Some(encoder)
    })
}

fn run_ffmpeg(mut cmd: Command, description: &str, output_path: &Path) -> Result<(), String> {
    let output = cmd
        .output()
        .map_err(|e| format!("failed to spawn ffmpeg: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output_path.exists() {
            let _ = fs::remove_file(output_path);
        }
        Err(format!(
            "ffmpeg exited with {} for {description}: {stderr}",
            output.status
        ))
    }
}

fn run_ffmpeg_or_panic(cmd: Command, description: &str, output_path: &Path) {
    if let Err(e) = run_ffmpeg(cmd, description, output_path) {
        panic!("Failed to generate {description}: {e}");
    }
}
