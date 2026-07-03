use std::fs;

use assert_cmd::Command;
use clap::Parser;
use predicates::prelude::*;
use tempfile::TempDir;

mod fixtures;
use fixtures::audio_fixture;

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
        .env("CI", "false")
        .assert()
        .failure()
        .stderr(predicate::str::contains("output file already exists"));
}
