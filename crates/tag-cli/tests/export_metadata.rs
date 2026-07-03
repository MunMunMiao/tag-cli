use std::fs;
use std::path::PathBuf;
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
        stderr.to_lowercase().contains("readme.txt")
            || stderr.to_lowercase().contains("skipped")
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
        .args([
            "apply",
            "-m",
            manifest_path.to_str().unwrap(),
            "--dry-run",
        ])
        .output()
        .expect("failed to run apply");

    assert!(
        apply_output.status.success(),
        "apply --dry-run failed on exported manifest: {}",
        String::from_utf8_lossy(&apply_output.stderr)
    );
}
