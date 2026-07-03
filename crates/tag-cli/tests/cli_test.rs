use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

mod fixtures;
use fixtures::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn copy_fixture_to_tmp(name: &str, tmp: &TempDir) -> PathBuf {
    let src = audio_fixture(name);
    let dst = tmp.path().join(name);
    fs::copy(&src, &dst).unwrap();
    dst
}

fn audio_fixtures() -> Vec<&'static str> {
    vec![
        "sample_flac.flac",
        "sample_mp3.mp3",
        "sample_m4a.m4a",
        "sample_ogg.ogg",
        "sample_opus.opus",
        "sample_wav.wav",
        "sample_aiff.aiff",
    ]
}

fn cover_image_fixtures() -> Vec<&'static str> {
    vec![
        "cover_jpg.jpg",
        "cover_png.png",
        "cover_gif.gif",
        "cover_bmp.bmp",
        "cover_webp.webp",
        "cover_tiff.tiff",
    ]
}

fn run_info(input: &Path) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["info", "-i", input.to_str().unwrap()]);
    cmd.assert().success()
}

fn run_set(input: &Path, tag: &str) {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["set", "-i", input.to_str().unwrap(), "-y", tag]);
    cmd.assert().success();
}

fn run_get(input: &Path, key: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", input.to_str().unwrap(), key]);
    cmd.assert().success()
}

fn run_cover_set(input: &Path, cover: &Path) {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        cover.to_str().unwrap(),
    ]);
    cmd.assert().success();
}

fn run_cover_clear(input: &Path) {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["cover", "clear", "-i", input.to_str().unwrap(), "-y"]);
    cmd.assert().success();
}

fn cmd() -> Command {
    Command::cargo_bin("tag-cli").unwrap()
}

// ---------------------------------------------------------------------------
// Existing tests
// ---------------------------------------------------------------------------

#[test]
fn test_info_reads_metadata() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "info",
        "-i",
        audio_fixture("sample_flac.flac").to_str().unwrap(),
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TITLE"));
}

#[test]
fn test_set_updates_tag() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "TITLE=New Title",
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", input.to_str().unwrap(), "TITLE"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("New Title"));
}

#[test]
fn test_set_replace_clears_other_tags() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    run_set(&input, "ARTIST=TestArtist");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "--replace",
        "TITLE=NewTitle",
    ]);
    cmd.assert().success();

    run_get(&input, "TITLE").stdout(predicate::str::contains("NewTitle"));
    run_get(&input, "ARTIST").stdout(predicate::str::contains("TestArtist").not());
}

#[test]
fn test_set_replace_dry_run_shows_cleared_tags() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    run_set(&input, "ARTIST=TestArtist");

    let assert = cmd()
        .args([
            "set",
            "-i",
            input.to_str().unwrap(),
            "--dry-run",
            "--replace",
            "TITLE=NewTitle",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Would update:"));
    assert!(stdout.contains("TITLE:"));
    assert!(stdout.contains("ARTIST:"));
    assert!(stdout.contains("(cleared)"));

    run_get(&input, "TITLE").stdout(predicate::str::contains("NewTitle").not());
    run_get(&input, "ARTIST").stdout(predicate::str::contains("TestArtist"));
}

#[test]
fn test_unsupported_image_format_errors() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        audio_fixture("sample_flac.flac").to_str().unwrap(),
        "-y",
        audio_fixture("sample_flac.flac").to_str().unwrap(),
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unsupported image format"));
}

#[test]
fn test_cover_set_and_get_with_jpg() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover_out = tmp.path().join("cover_out.jpg");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "get",
        "-i",
        input.to_str().unwrap(),
        "-o",
        cover_out.to_str().unwrap(),
    ]);
    cmd.assert().success();

    assert!(cover_out.exists());
    assert!(fs::metadata(&cover_out).unwrap().len() > 0);
}

#[test]
fn test_clear_all_tags() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["clear", "-i", input.to_str().unwrap(), "-y", "--all"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", input.to_str().unwrap(), "TITLE"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Original Title").not());
}

#[test]
fn test_clear_all_removes_cover() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let cover = image_fixture("cover_jpg.jpg");
    run_cover_set(&input, &cover);
    run_info(&input).stdout(predicate::str::contains("image/jpeg"));

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["clear", "-i", input.to_str().unwrap(), "-y", "--all"]);
    cmd.assert().success();

    run_info(&input).stdout(predicate::str::contains("image/jpeg").not());
}

#[test]
fn test_apply_manifest() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: Applied Title\n",
            input.file_name().unwrap().to_str().unwrap()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", input.to_str().unwrap(), "TITLE"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Applied Title"));
}

#[test]
fn apply_accepts_manifest_short_and_alias() {
    let tmp = TempDir::new().unwrap();
    let input_m = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let input_f = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);

    let manifest_m = tmp.path().join("manifest_m.yaml");
    fs::write(
        &manifest_m,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: M Title\n",
            input_m.file_name().unwrap().to_str().unwrap()
        ),
    )
    .unwrap();

    let manifest_f = tmp.path().join("manifest_f.yaml");
    fs::write(
        &manifest_f,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: F Title\n",
            input_f.file_name().unwrap().to_str().unwrap()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest_m.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    run_get(&input_m, "TITLE").stdout(predicate::str::contains("M Title"));

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-f", manifest_f.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    run_get(&input_f, "TITLE").stdout(predicate::str::contains("F Title"));
}

#[test]
fn set_writes_status_to_stderr_and_data_to_stdout() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);

    let assert = cmd()
        .args(["set", "-i", input.to_str().unwrap(), "-y", "TITLE=New"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("saved to"));
    assert!(stdout.is_empty());
}

#[test]
fn test_set_and_get_mp3() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "TITLE=MP3 Title",
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", input.to_str().unwrap(), "TITLE"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("MP3 Title"));
}

#[test]
fn test_info_reads_mp3_metadata() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "info",
        "-i",
        audio_fixture("sample_mp3.mp3").to_str().unwrap(),
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TITLE"));
}

#[test]
fn test_clear_specific_tag() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "ARTIST=Test Artist",
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["clear", "-i", input.to_str().unwrap(), "-y", "TITLE"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", input.to_str().unwrap(), "TITLE"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Original Title").not());

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", input.to_str().unwrap(), "ARTIST"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test Artist"));
}

#[test]
fn test_cover_clear() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["cover", "clear", "-i", input.to_str().unwrap(), "-y"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["info", "-i", input.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pictures:"));
}

// ---------------------------------------------------------------------------
// New fixture-based tests
// ---------------------------------------------------------------------------

#[test]
fn test_info_reads_metadata_for_all_audio_fixtures() {
    for fixture in audio_fixtures() {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);
        run_info(&input)
            .stdout(predicate::str::contains("Audio:"))
            .stdout(predicate::str::contains("Tags:"))
            .stdout(predicate::str::contains("Pictures:"));
    }
}

#[test]
fn test_set_and_get_title_for_all_audio_fixtures() {
    for fixture in audio_fixtures() {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);
        run_set(&input, "TITLE=New Title");
        run_get(&input, "TITLE").stdout(predicate::str::contains("New Title"));
    }
}

#[test]
fn test_cover_set_and_clear_for_all_audio_fixtures() {
    let cover = image_fixture("cover_jpg.jpg");
    for fixture in audio_fixtures() {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);

        run_cover_set(&input, &cover);
        run_info(&input).stdout(predicate::str::contains("image/jpeg"));

        run_cover_clear(&input);
        run_info(&input).stdout(predicate::str::contains("image/jpeg").not());
    }
}

#[test]
fn test_cover_from_each_image_format_on_flac() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    for image_fixture_name in cover_image_fixtures() {
        let cover = image_fixture(image_fixture_name);
        run_cover_set(&input, &cover);
        run_info(&input).stdout(predicate::str::contains("Front Cover"));
        run_cover_clear(&input);
        run_info(&input).stdout(predicate::str::contains("Front Cover").not());
    }
}

#[test]
fn test_write_to_new_output_file() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let output = tmp.path().join("output.flac");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "-y",
        "TITLE=Output Title",
    ]);
    cmd.assert().success();

    assert!(output.exists());
    assert!(fs::metadata(&output).unwrap().len() > 0);

    run_get(&output, "TITLE").stdout(predicate::str::contains("Output Title"));
    run_get(&input, "TITLE").stdout(predicate::str::contains("Output Title").not());
}

// ---------------------------------------------------------------------------
// CLI error / edge-case tests for 100% coverage
// ---------------------------------------------------------------------------

#[test]
fn test_set_invalid_key_value_format() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "NOT_A_KEY_VALUE",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("expected KEY=VALUE"));
}

#[test]
fn test_info_json_format() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "info",
        "-i",
        audio_fixture("sample_flac.flac").to_str().unwrap(),
        "--format",
        "json",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"file\""));
}

#[test]
fn test_info_yaml_format() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "info",
        "-i",
        audio_fixture("sample_flac.flac").to_str().unwrap(),
        "--format",
        "yaml",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("file:"));
}

#[test]
fn test_get_json_format() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "get",
        "-i",
        audio_fixture("sample_flac.flac").to_str().unwrap(),
        "TITLE",
        "--format",
        "json",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"TITLE\""));
}

#[test]
fn test_get_yaml_format() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "get",
        "-i",
        audio_fixture("sample_flac.flac").to_str().unwrap(),
        "TITLE",
        "--format",
        "yaml",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TITLE:"));
}

#[test]
fn test_cover_quality_out_of_range() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    for quality in ["0", "101"] {
        let mut cmd = Command::cargo_bin("tag-cli").unwrap();
        cmd.args([
            "cover",
            "set",
            "-i",
            input.to_str().unwrap(),
            "-y",
            "--cover-quality",
            quality,
            image_fixture("cover_jpg.jpg").to_str().unwrap(),
        ]);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("cover quality must be between"));
    }
}

#[test]
fn test_cover_format_png() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "--cover-format",
        "png",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["info", "-i", input.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("image/png"));
}

#[test]
fn test_no_process_cover() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "--no-process-cover",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();
}

#[test]
fn test_set_same_input_output_error() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-o",
        input.to_str().unwrap(),
        "-y",
        "TITLE=Same",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("output path cannot be the same"));
}

#[test]
fn test_set_requires_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.env("CI", "false")
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=NoConfirm"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("in-place modification requires"));
}

#[test]
fn test_clear_requires_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.env("CI", "false")
        .args(["clear", "-i", input.to_str().unwrap(), "TITLE"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("in-place modification requires"));
}

#[test]
fn test_apply_missing_manifest() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", "/does/not/exist.yaml", "-y"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("failed to read manifest"));
}

#[test]
fn test_apply_invalid_manifest() {
    let tmp = TempDir::new().unwrap();
    let manifest = tmp.path().join("manifest.yaml");
    fs::write(&manifest, "not: [ valid yaml").unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("failed to read manifest"));
}

#[test]
fn test_apply_invalid_glob() {
    let tmp = TempDir::new().unwrap();
    let manifest = tmp.path().join("manifest.yaml");
    fs::write(&manifest, "paths:\n  - \"[\"\n").unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid glob"));
}

#[test]
fn test_apply_literal_file_path() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "paths:\n  - {}\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));
}

#[test]
fn test_apply_absolute_path_pattern() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!("paths:\n  - {}\n", input.to_str().unwrap(),),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));
}

#[test]
fn test_apply_valid_glob_pattern() {
    let tmp = TempDir::new().unwrap();
    let a = tmp.path().join("track_a.mp3");
    let b = tmp.path().join("track_b.mp3");
    fs::copy(audio_fixture("sample_mp3.mp3"), &a).unwrap();
    fs::copy(audio_fixture("sample_mp3.mp3"), &b).unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(&manifest, "paths:\n  - \"track_*.mp3\"\n").unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 2"));
}

#[test]
fn test_apply_missing_path_pattern() {
    let tmp = TempDir::new().unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(&manifest, "paths:\n  - does_not_exist.mp3\n").unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 0"));
}

#[test]
fn test_apply_fail_fast() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let missing = tmp.path().join("missing.flac");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n  - path: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
            missing.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "apply",
        "-m",
        manifest.to_str().unwrap(),
        "-y",
        "--fail-fast",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Success: 1"))
        .stderr(predicate::str::contains("Failures: 1"))
        .stderr(predicate::str::contains("file(s) failed to apply"));
}

#[test]
fn test_apply_manifest_invalid_image_quality_zero() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "image_processing:\n  quality: 0\nfiles:\n  - path: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "manifest image quality must be between",
    ));
}

#[test]
fn test_apply_manifest_unsupported_image_format() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "image_processing:\n  format: gif\nfiles:\n  - path: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert().success();
}

#[test]
fn test_apply_fail_fast_breaks_after_first_failure() {
    let tmp = TempDir::new().unwrap();
    let _input = tmp.path().join("missing.flac");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "files:\n  - path: missing.flac\n  - path: missing2.flac\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "apply",
        "-m",
        manifest.to_str().unwrap(),
        "-y",
        "--fail-fast",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failures: 1"));
}

#[test]
fn test_verbose_flag_outputs_processing_logs_to_stderr() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["-v", "info", "-i", input.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("executing step: ReadMetadata"))
        .stderr(predicate::str::contains("reading metadata from"))
        .stderr(predicate::str::contains("executing step: FormatOutput"));
}

#[test]
fn test_cover_format_jpeg() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);
    let cover = image_fixture("cover_jpg.jpg");
    let output = tmp.path().join("out.mp3");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        cover.to_str().unwrap(),
        "--cover-format",
        "jpeg",
        "-o",
        output.to_str().unwrap(),
        "-y",
    ]);
    cmd.assert().success();
}

#[test]
fn test_cover_quality_zero_errors() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);
    let cover = image_fixture("cover_jpg.jpg");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        cover.to_str().unwrap(),
        "--cover-quality",
        "0",
        "-y",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cover quality must be between"));
}

#[test]
fn test_apply_with_defaults_and_cover() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover = image_fixture("cover_jpg.jpg");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "defaults:\n  ALBUM: Default Album\nfiles:\n  - path: {}\n    cover: {}\n    tags:\n      TITLE: File Title\n",
            input.file_name().unwrap().to_str().unwrap(),
            cover.to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));
}

#[test]
fn test_apply_manifest_image_processing() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "image_processing:\n  format: png\n  max_size: 500\n  max_file_size: 500\n  quality: 80\nfiles:\n  - path: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert().success();
}

#[test]
fn test_apply_manifest_invalid_image_quality() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "image_processing:\n  quality: 101\nfiles:\n  - path: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "manifest image quality must be between",
    ));
}

#[test]
fn test_apply_absolute_path() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: Abs Title\n",
            input.to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    run_get(&input, "TITLE").stdout(predicate::str::contains("Abs Title"));
}

#[test]
fn test_cover_get_no_cover_error() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let output = tmp.path().join("cover.jpg");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "get",
        "-i",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("no cover found"));
}

#[test]
fn test_set_unsupported_key() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "UNKNOWNKEY=Value",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unsupported tag key"));
}

#[test]
fn test_clear_unsupported_key() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["clear", "-i", input.to_str().unwrap(), "-y", "UNKNOWNKEY"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unsupported tag key"));
}

#[test]
fn test_apply_continue_after_failure() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let missing = tmp.path().join("missing.flac");
    let missing2 = tmp.path().join("missing2.flac");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n  - path: {}\n  - path: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
            missing.file_name().unwrap().to_str().unwrap(),
            missing2.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Success: 1"))
        .stderr(predicate::str::contains("Failures: 2"))
        .stderr(predicate::str::contains("file(s) failed to apply"));
}

#[test]
fn test_apply_cover_quality_zero_errors() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover = image_fixture("cover_jpg.jpg");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    cover: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
            cover.to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "apply",
        "-m",
        manifest.to_str().unwrap(),
        "-y",
        "--cover-quality",
        "0",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cover quality must be between"));
}

#[test]
fn test_apply_manifest_format_jpeg() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover = image_fixture("cover_jpg.jpg");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "image_processing:\n  format: jpeg\nfiles:\n  - path: {}\n    cover: {}\n",
            input.file_name().unwrap().to_str().unwrap(),
            cover.to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));
}

#[test]
fn apply_expands_glob_paths() {
    let tmp = TempDir::new().unwrap();
    let mp3 = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);
    let flac = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "defaults:\n  TITLE: Glob Title\npaths:\n  - '*.mp3'\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    run_get(&mp3, "TITLE").stdout(predicate::str::contains("Glob Title"));
    run_get(&flac, "TITLE").stdout(predicate::str::contains("Glob Title").not());
}

#[test]
fn apply_expands_directory_paths() {
    let tmp = TempDir::new().unwrap();
    let tracks = tmp.path().join("tracks");
    fs::create_dir(&tracks).unwrap();
    let mp3 = tracks.join("sample_mp3.mp3");
    let flac = tracks.join("sample_flac.flac");
    fs::copy(audio_fixture("sample_mp3.mp3"), &mp3).unwrap();
    fs::copy(audio_fixture("sample_flac.flac"), &flac).unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "defaults:\n  TITLE: Dir Title\npaths:\n  - tracks\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 2"));

    run_get(&mp3, "TITLE").stdout(predicate::str::contains("Dir Title"));
    run_get(&flac, "TITLE").stdout(predicate::str::contains("Dir Title"));
}

#[test]
fn apply_expands_recursive_directory_paths() {
    let tmp = TempDir::new().unwrap();
    let tracks = tmp.path().join("tracks");
    let sub = tracks.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let mp3 = tracks.join("sample_mp3.mp3");
    let flac = sub.join("sample_flac.flac");
    fs::copy(audio_fixture("sample_mp3.mp3"), &mp3).unwrap();
    fs::copy(audio_fixture("sample_flac.flac"), &flac).unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "defaults:\n  TITLE: Rec Title\nrecursive: true\npaths:\n  - tracks\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 2"));

    run_get(&mp3, "TITLE").stdout(predicate::str::contains("Rec Title"));
    run_get(&flac, "TITLE").stdout(predicate::str::contains("Rec Title"));
}

#[test]
fn apply_non_recursive_directory_ignores_subdirectories() {
    let tmp = TempDir::new().unwrap();
    let tracks = tmp.path().join("tracks");
    let sub = tracks.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let mp3 = tracks.join("sample_mp3.mp3");
    let flac = sub.join("sample_flac.flac");
    fs::copy(audio_fixture("sample_mp3.mp3"), &mp3).unwrap();
    fs::copy(audio_fixture("sample_flac.flac"), &flac).unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "defaults:\n  TITLE: NonRec Title\nrecursive: false\npaths:\n  - tracks\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    run_get(&mp3, "TITLE").stdout(predicate::str::contains("NonRec Title"));
    run_get(&flac, "TITLE").stdout(predicate::str::contains("NonRec Title").not());
}

#[test]
fn test_info_invalid_input_errors() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["info", "-i", "/does/not/exist.flac"]);
    cmd.assert().failure();
}

#[test]
fn test_get_invalid_input_errors() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["get", "-i", "/does/not/exist.flac", "TITLE"]);
    cmd.assert().failure();
}

#[test]
fn test_cover_get_invalid_input_errors() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("cover.jpg");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "get",
        "-i",
        "/does/not/exist.flac",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().failure();
}

#[test]
fn test_cover_get_write_failure_errors() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "get",
        "-i",
        input.to_str().unwrap(),
        "-o",
        "/does/not/exist_dir/cover.jpg",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("io error"));
}

#[test]
fn test_cover_set_invalid_input_errors() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        "/does/not/exist.flac",
        "-y",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().failure();
}

#[test]
fn test_cover_clear_invalid_input_errors() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["cover", "clear", "-i", "/does/not/exist.flac", "-y"]);
    cmd.assert().failure();
}

#[test]
fn test_apply_dry_run() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: Dry Title\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y", "--dry-run"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Skipped: 1"));

    run_get(&input, "TITLE").stdout(predicate::str::contains("Dry Title").not());
}

#[test]
fn test_apply_dry_run_shows_field_diff() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: Dry Title\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y", "--dry-run"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Would update:"));
    assert!(stdout.contains("TITLE:"));
}

#[test]
fn test_readme_manifest_example_dry_run() {
    let tmp = TempDir::new().unwrap();

    // Copy fixtures to the file names used in the README example.
    let intro = tmp.path().join("01-intro.mp3");
    fs::copy(audio_fixture("sample_mp3.mp3"), &intro).unwrap();

    let main_track = tmp.path().join("02-main.flac");
    fs::copy(audio_fixture("sample_flac.flac"), &main_track).unwrap();

    let artwork = tmp.path().join("artwork.jpg");
    fs::copy(image_fixture("cover_jpg.jpg"), &artwork).unwrap();

    // Write the README manifest example verbatim.
    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        r#"defaults:
  ARTIST: "Example Artist"
  ALBUM: "Example Album"
  DATE: "2026"

files:
  - path: "01-intro.mp3"
    tags:
      TITLE: "Intro"
      TRACKNUMBER: "1"
    cover: "artwork.jpg"

  - path: "02-main.flac"
    tags:
      TITLE: "Main Track"
      TRACKNUMBER: "2"
    cover: "artwork.jpg"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y", "--dry-run"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Skipped: 2"));
}

#[test]
fn test_apply_dry_run_shows_save_message() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: Dry Title\n",
            input.file_name().unwrap().to_str().unwrap(),
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y", "--dry-run"]);
    let assert = cmd.assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("[dry-run] would save to"));
}

// ---------------------------------------------------------------------------
// --picture-type tests
// ---------------------------------------------------------------------------

#[test]
fn test_cover_set_picture_type() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "--picture-type",
        "Back Cover",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["info", "-i", input.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Back Cover"));
}

#[test]
fn test_cover_get_picture_type() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover_out = tmp.path().join("cover_out.jpg");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        "--picture-type",
        "Back Cover",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "get",
        "-i",
        input.to_str().unwrap(),
        "-o",
        cover_out.to_str().unwrap(),
        "--picture-type",
        "Back Cover",
    ]);
    cmd.assert().success();

    assert!(cover_out.exists());
    assert!(fs::metadata(&cover_out).unwrap().len() > 0);
}

#[test]
fn test_cover_get_picture_type_no_match_errors() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover_out = tmp.path().join("cover_out.jpg");

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "set",
        "-i",
        input.to_str().unwrap(),
        "-y",
        image_fixture("cover_jpg.jpg").to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args([
        "cover",
        "get",
        "-i",
        input.to_str().unwrap(),
        "-o",
        cover_out.to_str().unwrap(),
        "--picture-type",
        "Back Cover",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("no cover found"));
}

#[test]
fn test_apply_manifest_picture_type() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover = image_fixture("cover_jpg.jpg");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    cover: {}\n    picture_type: Back Cover\n",
            input.file_name().unwrap().to_str().unwrap(),
            cover.to_str().unwrap()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["apply", "-m", manifest.to_str().unwrap(), "-y"]);
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["info", "-i", input.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Back Cover"));
}

// ---------------------------------------------------------------------------
// list-keys tests
// ---------------------------------------------------------------------------

#[test]
fn test_list_keys_default_table() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["list-keys"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TITLE"))
        .stdout(predicate::str::contains("ARTIST"));
}

#[test]
fn test_list_keys_json_format() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["list-keys", "--format", "json"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"TITLE\""))
        .stdout(predicate::str::contains("["));
}

#[test]
fn test_list_keys_yaml_format() {
    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["list-keys", "--format", "yaml"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TITLE"))
        .stdout(predicate::str::contains("- "));
}

// ---------------------------------------------------------------------------
// Help output tests
// ---------------------------------------------------------------------------

#[test]
fn top_level_help_groups_commands_by_workflow() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Inspect commands:"))
        .stdout(predicate::str::contains("info       Show all metadata"))
        .stdout(predicate::str::contains(
            "get        Read selected tag values",
        ))
        .stdout(predicate::str::contains(
            "list-keys  List supported tag keys",
        ))
        .stdout(predicate::str::contains("Edit commands:"))
        .stdout(predicate::str::contains("set        Set tag values"))
        .stdout(predicate::str::contains(
            "clear      Clear selected or all tags",
        ))
        .stdout(predicate::str::contains(
            "cover      Manage embedded cover art",
        ))
        .stdout(predicate::str::contains("Batch commands:"))
        .stdout(predicate::str::contains("apply      Apply a YAML manifest"))
        .stdout(predicate::str::contains(
            "export     Export metadata from audio files",
        ))
        .stdout(predicate::str::contains("Utility commands:"))
        .stdout(predicate::str::contains(
            "update     Update tag-cli to the latest release",
        ))
        .stdout(predicate::str::contains("Use \"tag-cli <COMMAND> --help\""));
}

#[test]
fn command_help_uses_comment_examples() {
    cmd()
        .args(["set", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Examples:"))
        .stdout(predicate::str::contains(
            "# Preview a tag edit before writing",
        ))
        .stdout(predicate::str::contains(
            "tag-cli set -i song.mp3 --dry-run",
        ));
}

#[test]
fn info_help_includes_comment_examples() {
    let assert = cmd().args(["info", "--help"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("# Show all metadata in table form"));
    assert!(stdout.contains("tag-cli info -i song.mp3"));
}

#[test]
fn cover_subcommand_help_includes_descriptions() {
    let assert = cmd().args(["cover", "--help"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Extract embedded cover art"));
    assert!(stdout.contains("Set embedded cover art from an image"));
    assert!(stdout.contains("Remove embedded cover art"));
    assert!(stdout.contains("# Extract embedded cover art"));
}

#[test]
fn apply_help_includes_comment_examples() {
    let assert = cmd().args(["apply", "--help"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("# Apply manifest changes after confirmation is explicit"));
    assert!(stdout.contains("tag-cli apply -m manifest.yaml -y"));
}

// ---------------------------------------------------------------------------
// Dry-run and confirmation tests
// ---------------------------------------------------------------------------

#[test]
fn test_set_dry_run_shows_diff_and_does_not_modify() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let assert = cmd()
        .args([
            "set",
            "-i",
            input.to_str().unwrap(),
            "--dry-run",
            "TITLE=Dry Title",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stdout.contains("Would update:"));
    assert!(stdout.contains("TITLE:"));
    assert!(stderr.contains("[dry-run] would save to"));

    run_get(&input, "TITLE").stdout(predicate::str::contains("Dry Title").not());
}

#[test]
fn test_clear_dry_run_shows_diff_and_does_not_modify() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    run_set(&input, "TITLE=New Title");

    let assert = cmd()
        .args(["clear", "-i", input.to_str().unwrap(), "--dry-run", "TITLE"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Would update:"));
    assert!(stdout.contains("TITLE: Some([\"New Title\"]) -> (cleared)"));

    run_get(&input, "TITLE").stdout(predicate::str::contains("New Title"));
}

#[test]
fn test_cover_set_dry_run_shows_diff_and_does_not_modify() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover = image_fixture("cover_jpg.jpg");

    let assert = cmd()
        .args([
            "cover",
            "set",
            "-i",
            input.to_str().unwrap(),
            "--dry-run",
            cover.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stdout.contains("Would update:"));
    assert!(stdout.contains("cover: (old) -> (new processed cover)"));
    assert!(stderr.contains("[dry-run] would save to"));
}

#[test]
fn test_cover_clear_dry_run_shows_diff_and_does_not_modify() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover = image_fixture("cover_jpg.jpg");
    run_cover_set(&input, &cover);

    let assert = cmd()
        .args(["cover", "clear", "-i", input.to_str().unwrap(), "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stdout.contains("Would update:"));
    assert!(stdout.contains("cover: (present) -> (removed)"));
    assert!(stderr.contains("[dry-run] would save to"));
}

#[test]
fn test_set_requires_confirmation_with_hint() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    cmd()
        .env("CI", "false")
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=Foo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("tag-cli set:"))
        .stderr(predicate::str::contains("--dry-run"));
}

#[test]
fn test_env_tag_cli_yes_bypasses_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    cmd()
        .env("TAG_CLI_YES", "1")
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=Env Title"])
        .assert()
        .success();

    run_get(&input, "TITLE").stdout(predicate::str::contains("Env Title"));
}

#[test]
fn test_ci_true_bypasses_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    cmd()
        .env("CI", "true")
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=Ci Title"])
        .assert()
        .success();

    run_get(&input, "TITLE").stdout(predicate::str::contains("Ci Title"));
}

#[test]
fn test_ci_false_does_not_bypass_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    cmd()
        .env("CI", "false")
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=Ci Title"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("tag-cli set:"));
}
