use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod fixtures;
use fixtures::{audio_fixture, image_fixture};

fn fixture_audio_dir() -> PathBuf {
    audio_fixture("sample_mp3.mp3")
        .parent()
        .unwrap()
        .to_path_buf()
}

fn tag_cli_in_audio_dir() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tag-cli"));
    cmd.current_dir(fixture_audio_dir());
    cmd
}

fn tag_cli_in_fixture_dir() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tag-cli"));
    cmd.current_dir(fixture_dir());
    cmd
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Runs `export metadata` against every generated fixture. Optional fixtures
/// that failed to generate are skipped; any generated fixture that tag-cli
/// cannot read causes the test to fail.
#[test]
fn export_metadata_all_generated_fixtures() {
    let fixtures = [
        "sample_flac.flac",
        "sample_mp3.mp3",
        "sample_mp2.mp2",
        "sample_m4a.m4a",
        "sample_ogg.ogg",
        "sample_opus.opus",
        "sample_wav.wav",
        "sample_aiff.aiff",
        "sample_oga.oga",
        "sample_spx.spx",
        "sample_wma.wma",
        "sample_wv.wv",
        "sample_mka.mka",
    ];

    for fixture in fixtures {
        let path = audio_fixture(fixture);
        if !path.exists() {
            // Optional fixture skipped because ffmpeg encoder was unavailable.
            continue;
        }

        let output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
            .args(["export", "metadata", "-i", path.to_str().unwrap()])
            .output()
            .expect("failed to run tag-cli");

        assert!(
            output.status.success(),
            "export metadata failed for {fixture}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("files:"),
            "expected manifest files section in output for {fixture}"
        );
        assert!(
            stdout.contains("path:"),
            "expected file path entry in output for {fixture}"
        );
    }
}

#[test]
fn export_metadata_yaml_stdout() {
    let output = tag_cli_in_audio_dir()
        .args(["export", "metadata", "-i", "sample_mp3.mp3"])
        .output()
        .expect("failed to run tag-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("files:"));
    assert!(stdout.contains("path:"));
    assert!(stdout.contains("TITLE: Original Title"));
}

#[test]
fn export_metadata_skips_non_audio_files() {
    let output = tag_cli_in_fixture_dir()
        .args(["export", "metadata", "-i", "*"])
        .output()
        .expect("failed to run tag-cli");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("readme.txt") || stderr.to_lowercase().contains("skipped")
    );
}

#[test]
fn export_metadata_sidecar_output() {
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tmp.path().join("meta");

    let output = tag_cli_in_audio_dir()
        .args([
            "export",
            "metadata",
            "-i",
            "sample_mp3.mp3",
            "-o",
            out_dir.to_str().unwrap(),
            "--per-file",
            "-y",
        ])
        .output()
        .expect("failed to run tag-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sidecar = out_dir.join("sample_mp3.metadata.yaml");
    assert!(
        sidecar.exists(),
        "expected sidecar at {}",
        sidecar.display()
    );

    let content = fs::read_to_string(&sidecar).unwrap();
    assert!(content.contains("files:"));
    assert!(content.contains("TITLE: Original Title"));
}

#[test]
fn export_metadata_rejects_format_flag() {
    let output = tag_cli_in_audio_dir()
        .args([
            "export",
            "metadata",
            "-i",
            "sample_mp3.mp3",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run tag-cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error: unexpected argument '--format' found")
            || stderr.contains("Found argument '--format'")
    );
}

#[test]
fn export_metadata_with_cover_extracts_front_cover() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest_path = tmp.path().join("album.yaml");
    let source = audio_fixture("sample_mp3.mp3");
    let with_cover = tmp.path().join("with_cover.mp3");
    fs::copy(&source, &with_cover).expect("copy fixture");

    // Embed a front cover.
    let cover = image_fixture("cover_png.png");
    let set_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args([
            "cover",
            "set",
            "-i",
            with_cover.to_str().unwrap(),
            "-y",
            "--no-process-cover",
            cover.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cover set");
    assert!(
        set_output.status.success(),
        "cover set failed: {}",
        String::from_utf8_lossy(&set_output.stderr)
    );

    // Export with cover extraction.
    let export_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args([
            "export",
            "metadata",
            "-i",
            with_cover.to_str().unwrap(),
            "-o",
            manifest_path.to_str().unwrap(),
            "--with-cover",
            "-y",
        ])
        .output()
        .expect("failed to run export metadata");
    assert!(
        export_output.status.success(),
        "export metadata failed: {}",
        String::from_utf8_lossy(&export_output.stderr)
    );

    let content = fs::read_to_string(&manifest_path).unwrap();
    assert!(content.contains("cover:"));
    assert!(content.contains("picture_type: Front Cover"));

    let cover_dir = tmp.path().join("album.covers");
    let cover_files: Vec<_> = fs::read_dir(&cover_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().unwrap().is_file())
        .collect();
    assert!(
        !cover_files.is_empty(),
        "expected extracted cover files in {}",
        cover_dir.display()
    );
}

#[test]
fn export_metadata_manifest_is_loadable_by_apply() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest_path = tmp.path().join("album.yaml");

    let output = tag_cli_in_audio_dir()
        .args([
            "export",
            "metadata",
            "-i",
            "sample_mp3.mp3",
            "-o",
            manifest_path.to_str().unwrap(),
            "--absolute-paths",
            "-y",
        ])
        .output()
        .expect("failed to run tag-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(manifest_path.exists());

    let apply_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args(["apply", "-m", manifest_path.to_str().unwrap(), "--dry-run"])
        .output()
        .expect("failed to run apply");

    assert!(
        apply_output.status.success(),
        "apply --dry-run failed on exported manifest: {}",
        String::from_utf8_lossy(&apply_output.stderr)
    );
}

fn generate_ffmpeg_audio(dst: &Path, codec_args: &[&str]) -> Result<(), String> {
    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("sine=frequency=1000:duration=1")
        .arg("-ar")
        .arg("44100")
        .arg("-ac")
        .arg("2")
        .args(codec_args)
        .arg(dst)
        .output()
        .map_err(|e| format!("failed to spawn ffmpeg: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        if dst.exists() {
            let _ = fs::remove_file(dst);
        }
        Err(format!(
            "ffmpeg failed for {}: {}",
            dst.display(),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[test]
fn export_metadata_round_trip_with_cover() {
    let tmp = tempfile::tempdir().unwrap();
    let source = audio_fixture("sample_mp3.mp3");
    let with_cover = tmp.path().join("with_cover.mp3");
    let round_trip = tmp.path().join("round_trip.mp3");
    let manifest_path = tmp.path().join("manifest.yaml");
    let cover = image_fixture("cover_jpg.jpg");

    fs::copy(&source, &with_cover).expect("copy source to with_cover");

    let set_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args([
            "set",
            "-i",
            with_cover.to_str().unwrap(),
            "-y",
            "TITLE=RoundTripTitle",
            "ARTIST=RoundTripArtist",
        ])
        .output()
        .expect("failed to run tag-cli set");
    assert!(
        set_output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&set_output.stderr)
    );

    let cover_set_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args([
            "cover",
            "set",
            "-i",
            with_cover.to_str().unwrap(),
            "-y",
            "--no-process-cover",
            cover.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cover set");
    assert!(
        cover_set_output.status.success(),
        "cover set failed: {}",
        String::from_utf8_lossy(&cover_set_output.stderr)
    );

    let export_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args([
            "export",
            "metadata",
            "-i",
            with_cover.to_str().unwrap(),
            "-o",
            manifest_path.to_str().unwrap(),
            "--with-cover",
            "--absolute-paths",
            "-y",
        ])
        .output()
        .expect("failed to run export metadata");
    assert!(
        export_output.status.success(),
        "export metadata failed: {}",
        String::from_utf8_lossy(&export_output.stderr)
    );

    let manifest_content = fs::read_to_string(&manifest_path).unwrap();
    assert!(manifest_content.contains("cover:"));
    assert!(manifest_content.contains("picture_type: Front Cover"));
    assert!(manifest_content.contains("RoundTripTitle"));

    fs::copy(&with_cover, &round_trip).expect("copy with_cover to round_trip");
    let old_path = manifest_content
        .lines()
        .find(|line| line.trim_start().starts_with("- path:"))
        .map(|line| line.trim_start().strip_prefix("- path:").unwrap().trim())
        .expect("manifest missing path line");
    let updated_manifest = manifest_content.replace(old_path, round_trip.to_str().unwrap());
    fs::write(&manifest_path, updated_manifest).unwrap();

    let apply_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args(["apply", "-m", manifest_path.to_str().unwrap(), "-y"])
        .output()
        .expect("failed to run apply");
    assert!(
        apply_output.status.success(),
        "apply failed: {}",
        String::from_utf8_lossy(&apply_output.stderr)
    );

    let get_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args(["get", "-i", round_trip.to_str().unwrap(), "-f", "json"])
        .output()
        .expect("failed to run get");
    assert!(
        get_output.status.success(),
        "get failed: {}",
        String::from_utf8_lossy(&get_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&get_output.stdout);
    assert!(stdout.contains("RoundTripTitle"));
    assert!(stdout.contains("RoundTripArtist"));

    let extracted_cover = tmp.path().join("extracted_cover.jpg");
    let cover_get_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
        .args([
            "cover",
            "get",
            "-i",
            round_trip.to_str().unwrap(),
            "-o",
            extracted_cover.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cover get");
    assert!(
        cover_get_output.status.success(),
        "cover get failed: {}",
        String::from_utf8_lossy(&cover_get_output.stderr)
    );
    assert_eq!(
        fs::read(&cover).unwrap(),
        fs::read(&extracted_cover).unwrap(),
        "extracted cover bytes do not match original"
    );
}

#[test]
fn export_metadata_recognizes_different_ffmpeg_encoders() {
    let tmp = tempfile::tempdir().unwrap();
    let cases: &[(&str, &[&str])] = &[
        (
            "vorbis.ogg",
            &["-c:a", "vorbis", "-strict", "experimental", "-q:a", "3"],
        ),
        ("libvorbis.ogg", &["-c:a", "libvorbis", "-q:a", "3"]),
        ("flac_ogg.oga", &["-c:a", "flac", "-f", "ogg"]),
        (
            "libopus.opus",
            &["-c:a", "libopus", "-b:a", "128k", "-ar", "48000"],
        ),
        ("libmp3lame.mp3", &["-c:a", "libmp3lame", "-q:a", "4"]),
    ];

    for (name, codec_args) in cases {
        let path = tmp.path().join(name);
        if let Err(e) = generate_ffmpeg_audio(&path, codec_args) {
            eprintln!("Skipping encoder case {name}: {e}");
            continue;
        }

        let export_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
            .args(["export", "metadata", "-i", path.to_str().unwrap()])
            .output()
            .expect("failed to run export metadata");
        assert!(
            export_output.status.success(),
            "export metadata failed for {name}: {}",
            String::from_utf8_lossy(&export_output.stderr)
        );

        let set_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
            .args([
                "set",
                "-i",
                path.to_str().unwrap(),
                "-y",
                "TITLE=EncoderTitle",
                "ARTIST=EncoderArtist",
            ])
            .output()
            .expect("failed to run set");
        assert!(
            set_output.status.success(),
            "set failed for {name}: {}",
            String::from_utf8_lossy(&set_output.stderr)
        );

        let get_output = Command::new(env!("CARGO_BIN_EXE_tag-cli"))
            .args(["get", "-i", path.to_str().unwrap(), "TITLE"])
            .output()
            .expect("failed to run get");
        assert!(
            get_output.status.success(),
            "get failed for {name}: {}",
            String::from_utf8_lossy(&get_output.stderr)
        );
        let stdout = String::from_utf8_lossy(&get_output.stdout);
        assert!(
            stdout.contains("EncoderTitle"),
            "expected EncoderTitle in get output for {name}: {stdout}"
        );
    }
}
