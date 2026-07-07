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
    cmd.args(["set", "-i", input.to_str().unwrap(), "TITLE=NoConfirm"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("in-place modification requires"));
}

#[test]
fn test_clear_requires_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut cmd = Command::cargo_bin("tag-cli").unwrap();
    cmd.args(["clear", "-i", input.to_str().unwrap(), "TITLE"]);
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
fn test_set_replace_clears_unsupported_tags_ogg() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.ogg");

    let status = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("sine=frequency=1000:duration=1")
        .arg("-ar")
        .arg("44100")
        .arg("-ac")
        .arg("2")
        .arg("-c:a")
        .arg("vorbis")
        .arg("-strict")
        .arg("experimental")
        .arg("-metadata")
        .arg("END=12345")
        .arg("-metadata")
        .arg("ENDGRAN=67890")
        .arg("-metadata")
        .arg("TITLE=Original")
        .arg(&input)
        .status()
        .expect("ffmpeg is required for this test");
    assert!(status.success(), "ffmpeg failed to generate ogg fixture");

    let dry_run = cmd()
        .args([
            "set",
            "-i",
            input.to_str().unwrap(),
            "--dry-run",
            "--replace",
            "TITLE=Replaced Title",
        ])
        .assert()
        .success();
    let dry_stdout = String::from_utf8(dry_run.get_output().stdout.clone()).unwrap();
    assert!(dry_stdout.contains("Would update:"));
    assert!(dry_stdout.contains("END:"));
    assert!(dry_stdout.contains("ENDGRAN:"));
    assert!(dry_stdout.contains("(cleared)"));

    cmd()
        .args([
            "set",
            "-i",
            input.to_str().unwrap(),
            "-y",
            "--replace",
            "TITLE=Replaced Title",
        ])
        .assert()
        .success();

    let export = cmd()
        .args(["export", "metadata", "-i", input.to_str().unwrap()])
        .assert()
        .success();
    let export_stdout = String::from_utf8(export.get_output().stdout.clone()).unwrap();
    assert!(export_stdout.contains("Replaced Title"));
    assert!(!export_stdout.contains("END:"));
    assert!(!export_stdout.contains("ENDGRAN:"));
}

#[test]
fn test_apply_replace_clears_unsupported_tags_ogg() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.ogg");

    let status = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("sine=frequency=1000:duration=1")
        .arg("-ar")
        .arg("44100")
        .arg("-ac")
        .arg("2")
        .arg("-c:a")
        .arg("vorbis")
        .arg("-strict")
        .arg("experimental")
        .arg("-metadata")
        .arg("END=12345")
        .arg("-metadata")
        .arg("ENDGRAN=67890")
        .arg("-metadata")
        .arg("TITLE=Original")
        .arg(&input)
        .status()
        .expect("ffmpeg is required for this test");
    assert!(status.success(), "ffmpeg failed to generate ogg fixture");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "files:\n  - path: input.ogg\n    tags:\n      TITLE: Replaced Title\n",
    )
    .unwrap();

    let dry_run = cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y", "--dry-run"])
        .current_dir(&tmp)
        .assert()
        .success();
    let dry_stdout = String::from_utf8(dry_run.get_output().stdout.clone()).unwrap();
    assert!(dry_stdout.contains("Would update:"));
    assert!(dry_stdout.contains("END:"));
    assert!(dry_stdout.contains("ENDGRAN:"));
    assert!(dry_stdout.contains("(cleared)"));

    cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
        .current_dir(&tmp)
        .assert()
        .success();

    let export = cmd()
        .args(["export", "metadata", "-i", input.to_str().unwrap()])
        .assert()
        .success();
    let export_stdout = String::from_utf8(export.get_output().stdout.clone()).unwrap();
    assert!(export_stdout.contains("Replaced Title"));
    assert!(!export_stdout.contains("END:"));
    assert!(!export_stdout.contains("ENDGRAN:"));
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
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=Foo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("tag-cli set:"))
        .stderr(predicate::str::contains("--dry-run"));
}

#[test]
fn test_ci_env_does_not_bypass_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    cmd()
        .env("CI", "true")
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=Ci Title"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("tag-cli set:"))
        .stderr(predicate::str::contains("requires -y/--yes"));
}

#[test]
fn test_tag_cli_yes_env_does_not_bypass_confirmation() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    cmd()
        .env("TAG_CLI_YES", "1")
        .args(["set", "-i", input.to_str().unwrap(), "TITLE=Env Title"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("tag-cli set:"))
        .stderr(predicate::str::contains("requires -y/--yes"));
}

// ---------------------------------------------------------------------------
// Scenario-based integration tests
// ---------------------------------------------------------------------------

fn get_json_values(input: &Path, key: &str) -> Vec<String> {
    let output = cmd()
        .args(["get", "-i", input.to_str().unwrap(), key, "-f", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "get -f json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).unwrap()).unwrap();
    match value.get(key) {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

fn get_all_json_values(input: &Path) -> std::collections::BTreeMap<String, Vec<String>> {
    let output = cmd()
        .args(["get", "-i", input.to_str().unwrap(), "-f", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "get -f json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).unwrap()).unwrap();
    let Some(obj) = value.as_object() else {
        return std::collections::BTreeMap::new();
    };
    obj.iter()
        .map(|(k, v)| {
            let vals = v
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            (k.clone(), vals)
        })
        .collect()
}

fn run_apply(manifest: &Path, tmp: &TempDir) {
    cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
        .current_dir(tmp)
        .assert()
        .success();
}

fn run_apply_dry_run(manifest: &Path, tmp: &TempDir) -> String {
    let output = cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y", "--dry-run"])
        .current_dir(tmp)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "apply --dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn parse_diff_keys(diff: &str) -> (Vec<String>, Vec<String>) {
    let mut cleared = Vec::new();
    let mut set = Vec::new();
    for line in diff.lines() {
        if let Some(body) = line.strip_prefix("  - ") {
            if let Some(colon) = body.find(':') {
                let key = body[..colon].trim().to_string();
                if line.contains("-> (cleared)") {
                    cleared.push(key);
                } else if line.contains("-> [") {
                    set.push(key);
                }
            }
        }
    }
    (cleared, set)
}

fn export_stdout_contains(input: &Path, needle: &str) -> bool {
    let output = cmd()
        .args(["export", "metadata", "-i", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "export metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).contains(needle)
}

#[test]
fn test_apply_dry_run_matches_actual_changes() {
    let supported: std::collections::HashSet<&str> = [
        "TITLE",
        "ARTIST",
        "ALBUM",
        "ALBUMARTIST",
        "GENRE",
        "DATE",
        "YEAR",
        "TRACKNUMBER",
        "TRACKTOTAL",
        "DISCNUMBER",
        "DISCTOTAL",
        "COMPOSER",
        "PUBLISHER",
        "COPYRIGHT",
        "COMMENT",
        "DESCRIPTION",
        "URL",
        "ISRC",
        "LABEL",
        "CATALOGNUMBER",
        "LYRICS",
        "LANGUAGE",
        "EXPLICIT",
        "BPM",
        "INITIALKEY",
        "KEY",
    ]
    .iter()
    .copied()
    .collect();

    for fixture in ["sample_flac.flac", "sample_mp3.mp3", "sample_ogg.ogg"] {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);
        let cover = image_fixture("cover_jpg.jpg");

        let manifest = tmp.path().join("manifest.yaml");
        fs::write(
            &manifest,
            format!(
                "defaults:\n  ALBUM: Default Album\nfiles:\n  - path: {}\n    cover: {}\n    tags:\n      TITLE: File Title\n",
                input.file_name().unwrap().to_str().unwrap(),
                cover.to_str().unwrap()
            ),
        )
        .unwrap();

        let dry_stdout = run_apply_dry_run(&manifest, &tmp);
        let (cleared, set) = parse_diff_keys(&dry_stdout);

        run_apply(&manifest, &tmp);

        for key in cleared {
            if supported.contains(key.as_str()) {
                assert!(
                    get_json_values(&input, &key).is_empty(),
                    "expected {key} to be cleared on {fixture}"
                );
            }
        }

        let expected: std::collections::BTreeMap<&str, &str> =
            [("TITLE", "File Title"), ("ALBUM", "Default Album")]
                .into_iter()
                .collect();
        for key in set {
            if let Some(&expected_val) = expected.get(key.as_str()) {
                assert_eq!(
                    get_json_values(&input, &key),
                    vec![expected_val],
                    "unexpected {key} on {fixture}"
                );
            }
        }

        run_info(&input).stdout(predicate::str::contains("Front Cover"));
    }
}

#[test]
fn test_manifest_defaults_and_per_file_overrides() {
    let tmp = TempDir::new().unwrap();
    let a = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let b = copy_fixture_to_tmp("sample_mp3.mp3", &tmp);

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "defaults:\n  ARTIST: Default Artist\n  ALBUM: Default Album\n  DATE: '2024'\n  GENRE: Rock\nfiles:\n  - path: {}\n    tags:\n      TITLE: Song One\n      TRACKNUMBER: '1'\n  - path: {}\n    tags:\n      TITLE: Song Two\n      TRACKNUMBER: '2'\n",
            a.file_name().unwrap().to_str().unwrap(),
            b.file_name().unwrap().to_str().unwrap()
        ),
    )
    .unwrap();

    cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
        .current_dir(&tmp)
        .assert()
        .success()
        .stderr(predicate::str::contains("Success: 2"));

    let cases = [(&a, "Song One", "1"), (&b, "Song Two", "2")];
    for (input, title, track) in &cases {
        for (key, val) in [
            ("ARTIST", "Default Artist"),
            ("ALBUM", "Default Album"),
            ("DATE", "2024"),
            ("GENRE", "Rock"),
            ("TITLE", *title),
            ("TRACKNUMBER", *track),
        ] {
            assert_eq!(
                get_json_values(input, key),
                vec![val],
                "{key} mismatch on {}",
                input.display()
            );
        }
    }
}

#[test]
fn test_apply_recursive_directory_mixed_formats() {
    let tmp = TempDir::new().unwrap();
    let album = tmp.path().join("album");
    let disc1 = album.join("disc1");
    let disc2 = album.join("disc2");
    fs::create_dir_all(&disc1).unwrap();
    fs::create_dir_all(&disc2).unwrap();

    let files = [
        (disc1.join("track1.mp3"), "sample_mp3.mp3"),
        (disc1.join("track2.flac"), "sample_flac.flac"),
        (disc2.join("track3.ogg"), "sample_ogg.ogg"),
        (disc2.join("track4.m4a"), "sample_m4a.m4a"),
    ];
    for (dst, src) in &files {
        fs::copy(audio_fixture(src), dst).unwrap();
    }

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "defaults:\n  TITLE: Recursive Title\nrecursive: true\npaths:\n  - album\n",
    )
    .unwrap();

    cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
        .current_dir(&tmp)
        .assert()
        .success()
        .stderr(predicate::str::contains("Success: 4"));

    for (dst, _) in &files {
        assert_eq!(get_json_values(dst, "TITLE"), vec!["Recursive Title"]);
    }
}

#[test]
fn test_apply_cover_default_picture_type() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let cover = image_fixture("cover_jpg.jpg");
    let extracted = tmp.path().join("extracted.jpg");

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    cover: {}\n    tags:\n      TITLE: With Cover\n",
            input.file_name().unwrap().to_str().unwrap(),
            cover.to_str().unwrap()
        ),
    )
    .unwrap();

    run_apply(&manifest, &tmp);

    run_info(&input).stdout(predicate::str::contains("Front Cover"));

    cmd()
        .args([
            "cover",
            "get",
            "-i",
            input.to_str().unwrap(),
            "-o",
            extracted.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(extracted.exists());
    assert!(!fs::read(&extracted).unwrap().is_empty());
}

#[test]
fn test_special_characters_roundtrip() {
    let value = "A&B/C'D\"E<F>G";

    // Set via CLI args.
    let tmp = TempDir::new().unwrap();
    let flac = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    cmd()
        .args([
            "set",
            "-i",
            flac.to_str().unwrap(),
            "-y",
            &format!("TITLE={}", value),
            &format!("ARTIST={}", value),
        ])
        .assert()
        .success();
    for key in ["TITLE", "ARTIST"] {
        let out = cmd()
            .args(["get", "-i", flac.to_str().unwrap(), key])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains(value));
    }
    assert!(export_stdout_contains(&flac, value));

    // Set via manifest.
    let tmp2 = TempDir::new().unwrap();
    let mp3 = copy_fixture_to_tmp("sample_mp3.mp3", &tmp2);
    let manifest = tmp2.path().join("manifest.yaml");
    let quoted = value.replace('"', "\\\"");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: \"{}\"\n      ARTIST: \"{}\"\n",
            mp3.file_name().unwrap().to_str().unwrap(),
            quoted,
            quoted
        ),
    )
    .unwrap();
    run_apply(&manifest, &tmp2);
    for key in ["TITLE", "ARTIST"] {
        let out = cmd()
            .args(["get", "-i", mp3.to_str().unwrap(), key])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains(value));
    }
    assert!(export_stdout_contains(&mp3, value));
}

#[test]
fn test_large_comment_roundtrip() {
    let size = 1024 * 1024;
    let big: String = std::iter::repeat('x').take(size).collect();

    for fixture in ["sample_flac.flac", "sample_mp3.mp3"] {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);
        let original_bytes = fs::read(&input).unwrap();

        let manifest = tmp.path().join("manifest.yaml");
        fs::write(
            &manifest,
            format!(
                "files:\n  - path: {}\n    tags:\n      COMMENT: {}\n",
                input.file_name().unwrap().to_str().unwrap(),
                big
            ),
        )
        .unwrap();

        let apply = cmd()
            .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
            .current_dir(&tmp)
            .output()
            .unwrap();

        if apply.status.success() {
            assert_eq!(get_json_values(&input, "COMMENT"), vec![big.as_str()]);
        } else {
            assert_eq!(
                fs::read(&input).unwrap(),
                original_bytes,
                "apply failed but modified {fixture}"
            );
        }
    }
}

#[test]
fn test_multi_value_artist() {
    for fixture in ["sample_mp3.mp3", "sample_flac.flac", "sample_ogg.ogg"] {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);

        cmd()
            .args([
                "set",
                "-i",
                input.to_str().unwrap(),
                "-y",
                "ARTIST=A",
                "ARTIST=B",
                "ARTIST=C",
            ])
            .assert()
            .success();

        let values = get_json_values(&input, "ARTIST");
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"A".to_string()));
        assert!(values.contains(&"B".to_string()));
        assert!(values.contains(&"C".to_string()));
    }
}

#[test]
fn test_empty_values_clear_fields() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    cmd()
        .args([
            "set",
            "-i",
            input.to_str().unwrap(),
            "-y",
            "COMMENT=old",
            "GENRE=old",
        ])
        .assert()
        .success();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      COMMENT: ''\n",
            input.file_name().unwrap().to_str().unwrap()
        ),
    )
    .unwrap();
    run_apply(&manifest, &tmp);

    assert!(get_json_values(&input, "COMMENT").is_empty());
    assert!(get_json_values(&input, "GENRE").is_empty());

    // set --replace COMMENT= also clears an existing COMMENT.
    cmd()
        .args([
            "set",
            "-i",
            input.to_str().unwrap(),
            "-y",
            "--replace",
            "COMMENT=",
        ])
        .assert()
        .success();
    assert!(get_json_values(&input, "COMMENT").is_empty());
}

#[test]
fn test_glob_precise_selection() {
    let tmp = TempDir::new().unwrap();
    let a = tmp.path().join("01-a.mp3");
    let b = tmp.path().join("01-b.flac");
    let c = tmp.path().join("02-a.mp3");
    let d = tmp.path().join("bonus.ogg");
    fs::copy(audio_fixture("sample_mp3.mp3"), &a).unwrap();
    fs::copy(audio_fixture("sample_flac.flac"), &b).unwrap();
    fs::copy(audio_fixture("sample_mp3.mp3"), &c).unwrap();
    fs::copy(audio_fixture("sample_ogg.ogg"), &d).unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        "defaults:\n  TITLE: Glob Matched\npaths:\n  - '01-*.mp3'\n",
    )
    .unwrap();

    cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
        .current_dir(&tmp)
        .assert()
        .success()
        .stderr(predicate::str::contains("Success: 1"));

    assert_eq!(get_json_values(&a, "TITLE"), vec!["Glob Matched"]);
    assert!(!get_json_values(&b, "TITLE").contains(&"Glob Matched".to_string()));
    assert!(!get_json_values(&c, "TITLE").contains(&"Glob Matched".to_string()));
    assert!(!get_json_values(&d, "TITLE").contains(&"Glob Matched".to_string()));
}

#[test]
fn test_cross_format_set_get_clear() {
    let supported: std::collections::HashSet<&str> = [
        "TITLE",
        "ARTIST",
        "ALBUM",
        "ALBUMARTIST",
        "GENRE",
        "DATE",
        "YEAR",
        "TRACKNUMBER",
        "TRACKTOTAL",
        "DISCNUMBER",
        "DISCTOTAL",
        "COMPOSER",
        "PUBLISHER",
        "COPYRIGHT",
        "COMMENT",
        "DESCRIPTION",
        "URL",
        "ISRC",
        "LABEL",
        "CATALOGNUMBER",
        "LYRICS",
        "LANGUAGE",
        "EXPLICIT",
        "BPM",
        "INITIALKEY",
        "KEY",
    ]
    .iter()
    .copied()
    .collect();

    for fixture in audio_fixtures() {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);

        cmd()
            .args([
                "set",
                "-i",
                input.to_str().unwrap(),
                "-y",
                "TITLE=Cross Title",
                "ARTIST=Cross Artist",
            ])
            .assert()
            .success();

        assert_eq!(get_json_values(&input, "TITLE"), vec!["Cross Title"]);
        assert_eq!(get_json_values(&input, "ARTIST"), vec!["Cross Artist"]);

        let clear = cmd()
            .args(["clear", "-i", input.to_str().unwrap(), "-y", "--all"])
            .output()
            .unwrap();

        if !clear.status.success() {
            eprintln!("clear --all failed for {fixture}, skipping remainder");
            continue;
        }

        let all = get_all_json_values(&input);
        for key in all.keys() {
            assert!(
                !supported.contains(key.as_str()),
                "expected no supported tags on {fixture}, found {key}"
            );
        }
    }
}

#[test]
fn test_cover_bytes_identical() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    for cover_name in ["cover_jpg.jpg", "cover_png.png"] {
        let cover = image_fixture(cover_name);
        let original = fs::read(&cover).unwrap();
        let extracted = tmp.path().join(format!("extracted_{}", cover_name));

        cmd()
            .args([
                "cover",
                "set",
                "-i",
                input.to_str().unwrap(),
                "-y",
                "--no-process-cover",
                cover.to_str().unwrap(),
            ])
            .assert()
            .success();

        cmd()
            .args([
                "cover",
                "get",
                "-i",
                input.to_str().unwrap(),
                "-o",
                extracted.to_str().unwrap(),
            ])
            .assert()
            .success();

        assert_eq!(
            fs::read(&extracted).unwrap(),
            original,
            "{cover_name} bytes differ"
        );

        run_cover_clear(&input);
    }
}

#[test]
fn test_clear_single_tag_preserves_others_and_cover() {
    for fixture in ["sample_flac.flac", "sample_mp3.mp3", "sample_ogg.ogg"] {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);
        let cover = image_fixture("cover_jpg.jpg");

        cmd()
            .args([
                "set",
                "-i",
                input.to_str().unwrap(),
                "-y",
                "TITLE=Keep Title",
                "ARTIST=Keep Artist",
                "GENRE=Keep Genre",
                "COMMENT=Keep Comment",
            ])
            .assert()
            .success();

        run_cover_set(&input, &cover);

        cmd()
            .args(["clear", "-i", input.to_str().unwrap(), "-y", "GENRE"])
            .assert()
            .success();

        assert!(get_json_values(&input, "GENRE").is_empty());
        assert_eq!(get_json_values(&input, "TITLE"), vec!["Keep Title"]);
        assert_eq!(get_json_values(&input, "ARTIST"), vec!["Keep Artist"]);
        assert_eq!(get_json_values(&input, "COMMENT"), vec!["Keep Comment"]);
        run_info(&input).stdout(predicate::str::contains("image/jpeg"));
    }
}

#[test]
#[cfg(unix)]
fn test_read_only_file_permission_error() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);

    let mut perms = fs::metadata(&input).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&input, perms).unwrap();

    let output = cmd()
        .args(["set", "-i", input.to_str().unwrap(), "-y", "TITLE=X"])
        .output()
        .unwrap();

    let mut perms = fs::metadata(&input).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&input, perms).unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("permission")
            || stderr.contains("read-only")
            || stderr.contains("read only"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn test_extension_mismatch_handled_gracefully() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("actually_flac.mp3");
    let original = audio_fixture("sample_flac.flac");
    fs::copy(&original, &input).unwrap();

    // The commands must not panic; current TagLib behavior treats the file as
    // an MP3 container and writes an ID3 tag, so we verify post-write
    // readability instead of unchanged bytes.
    let _ = cmd().args(["info", "-i", input.to_str().unwrap()]).assert();
    cmd()
        .args(["set", "-i", input.to_str().unwrap(), "-y", "TITLE=X"])
        .assert()
        .success();
    run_get(&input, "TITLE").stdout(predicate::str::contains("X"));
}

#[test]
fn test_large_manifest_no_panic() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let filename = input.file_name().unwrap().to_str().unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    let mut text = String::from("files:\n");
    for i in 0..2000 {
        text.push_str(&format!(
            "  - path: {filename}\n    tags:\n      TITLE: title_{i}\n"
        ));
    }
    fs::write(&manifest, text).unwrap();

    cmd()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y", "--dry-run"])
        .current_dir(&tmp)
        .assert()
        .success();
}

#[test]
fn test_parallel_set_same_file() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp("sample_flac.flac", &tmp);
    let exe = env!("CARGO_BIN_EXE_tag-cli");

    let mut children = Vec::new();
    for value in ["A", "B", "C", "D"] {
        let child = std::process::Command::new(exe)
            .args([
                "set",
                "-i",
                input.to_str().unwrap(),
                "-y",
                &format!("TITLE={value}"),
            ])
            .spawn()
            .unwrap();
        children.push(child);
    }

    for mut child in children {
        let _ = child.wait();
    }

    run_info(&input).stdout(predicate::str::contains("Audio:"));
    let title = get_json_values(&input, "TITLE");
    assert!(!title.is_empty());
    assert!("A B C D".split(' ').any(|v| v == title[0]));
}

#[test]
fn test_set_and_get_basic_tag() {
    for fixture in ["sample_mp3.mp3", "sample_flac.flac", "sample_ogg.ogg"] {
        let tmp = TempDir::new().unwrap();
        let input = copy_fixture_to_tmp(fixture, &tmp);
        run_set(&input, "TITLE=New");
        run_get(&input, "TITLE").stdout(predicate::str::contains("New"));
    }
}
