use std::ffi::OsString;
use std::fs;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use clap::Parser;
use predicates::prelude::*;
use tempfile::TempDir;

mod fixtures;
use fixtures::{audio_fixture, image_fixture};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn readme_fixture() -> PathBuf {
    fixture_dir().join("readme.txt")
}

fn copy_fixture_to_tmp<P: AsRef<Path>>(src: P, tmp: &TempDir, name: &str) -> PathBuf {
    let dst = tmp.path().join(name);
    fs::copy(src, &dst).unwrap();
    dst
}

fn assert_file_unchanged(path: &Path, expected: &[u8]) {
    let actual = fs::read(path).unwrap();
    assert_eq!(actual, expected, "{path:?} was unexpectedly modified",);
}

#[test]
fn run_with_propagates_export_metadata_overwrite_error() {
    let tmp = TempDir::new().unwrap();
    let input = audio_fixture("sample_flac.flac");
    let output = tmp.path().join("manifest.yaml");
    fs::write(&output, "existing content").unwrap();

    let cli = tag_cli::cli::Cli::parse_from([
        "tag-cli",
        "export",
        "metadata",
        "-i",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);

    let mut stdout = Vec::new();
    let result = tag_cli::run_with(cli, &mut stdout);
    assert!(
        result.is_err(),
        "expected export metadata to fail when output file exists without -y"
    );
}

#[test]
fn run_with_list_keys_succeeds() {
    let cli = tag_cli::cli::Cli::parse_from(["tag-cli", "list-keys"]);
    let mut stdout = Vec::new();
    let result = tag_cli::run_with(cli, &mut stdout);
    assert!(result.is_ok(), "expected list-keys to succeed: {result:?}");
}

#[test]
fn export_metadata_fails_when_output_exists_without_yes() {
    let tmp = TempDir::new().unwrap();
    let input = audio_fixture("sample_flac.flac");
    let output = tmp.path().join("manifest.yaml");
    fs::write(&output, "existing content").unwrap();

    Command::cargo_bin("tag-cli")
        .unwrap()
        .args([
            "export",
            "metadata",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("output file already exists"));
}

#[test]
fn truncated_flac_apply_returns_readable_error_without_modifying_file() {
    let tmp = TempDir::new().unwrap();
    let input = copy_fixture_to_tmp(audio_fixture("sample_flac.flac"), &tmp, "truncated.flac");

    let mut bytes = fs::read(&input).unwrap();
    // The generated fixture is small enough that 70% still leaves valid FLAC
    // metadata; truncate more aggressively so apply actually fails.
    let truncated_len = bytes.len() * 3 / 10;
    bytes.truncate(truncated_len);
    fs::write(&input, &bytes).unwrap();
    let truncated_bytes = fs::read(&input).unwrap();

    let manifest = tmp.path().join("manifest.yaml");
    fs::write(
        &manifest,
        format!(
            "files:\n  - path: {}\n    tags:\n      TITLE: X\n",
            input.to_str().unwrap()
        ),
    )
    .unwrap();

    let assert = Command::cargo_bin("tag-cli")
        .unwrap()
        .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
        .assert()
        .failure();

    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.to_lowercase().contains("corrupt")
            || stderr.to_lowercase().contains("unreadable")
            || stderr.to_lowercase().contains("valid")
            || stderr.contains("Could not read")
            || stderr.to_lowercase().contains("failed"),
        "expected readable error for truncated FLAC, got: {stderr}"
    );

    assert_file_unchanged(&input, &truncated_bytes);
}

#[test]
fn non_audio_file_with_audio_extension_is_rejected() {
    let tmp = TempDir::new().unwrap();
    // TagLib's MPEG parser is very lenient with .mp3 files, so the reliable
    // rejection case is non-FLAC content with a .flac extension.
    let fake_flac_image =
        copy_fixture_to_tmp(image_fixture("cover_jpg.jpg"), &tmp, "fake_image.flac");
    let fake_flac_text = copy_fixture_to_tmp(readme_fixture(), &tmp, "fake_text.flac");

    for fake in [&fake_flac_image, &fake_flac_text] {
        let original = fs::read(fake).unwrap();

        Command::cargo_bin("tag-cli")
            .unwrap()
            .args(["info", "-i", fake.to_str().unwrap()])
            .assert()
            .failure();
        assert_file_unchanged(fake, &original);

        Command::cargo_bin("tag-cli")
            .unwrap()
            .args(["set", "-i", fake.to_str().unwrap(), "-y", "TITLE=X"])
            .assert()
            .failure();
        assert_file_unchanged(fake, &original);

        let manifest = tmp.path().join(format!(
            "manifest_{}.yaml",
            fake.file_stem().unwrap().to_str().unwrap()
        ));
        fs::write(
            &manifest,
            format!(
                "files:\n  - path: {}\n    tags:\n      TITLE: X\n",
                fake.to_str().unwrap()
            ),
        )
        .unwrap();

        Command::cargo_bin("tag-cli")
            .unwrap()
            .args(["apply", "-m", manifest.to_str().unwrap(), "-y"])
            .assert()
            .failure();
        assert_file_unchanged(fake, &original);
    }
}

#[cfg(unix)]
#[test]
fn nul_byte_in_input_path_is_rejected_without_panic() {
    let nul_path = PathBuf::from(std::ffi::OsStr::from_bytes(b"foo\0bar"));

    for subcommand in ["set", "info"] {
        let args: Vec<OsString> = if subcommand == "set" {
            vec![
                OsString::from("tag-cli"),
                OsString::from("set"),
                OsString::from("-i"),
                nul_path.clone().into(),
                OsString::from("-y"),
                OsString::from("TITLE=X"),
            ]
        } else {
            vec![
                OsString::from("tag-cli"),
                OsString::from("info"),
                OsString::from("-i"),
                nul_path.clone().into(),
            ]
        };

        let cli = tag_cli::cli::Cli::parse_from(args);
        let result = tag_cli::run_with(cli, &mut Vec::new());
        assert!(
            result.is_err(),
            "expected {subcommand} to return an error for path with interior NUL"
        );
    }
}
