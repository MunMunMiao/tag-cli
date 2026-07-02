use std::path::PathBuf;
use std::process::Command;

mod fixtures;
use fixtures::audio_fixture;

fn tag_cli() -> Command {
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
            stdout.contains("\"records\""),
            "expected records in output for {fixture}"
        );
        assert!(
            stdout.contains("\"file_path\""),
            "expected file_path record for {fixture}"
        );
    }
}

#[test]
fn export_metadata_json_stdout() {
    let output = tag_cli()
        .args(["export", "metadata", "-i", "*.mp3"])
        .output()
        .expect("failed to run tag-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"generator\": \"tag-cli export metadata\""));
    assert!(stdout.contains("\"records\""));
}

#[test]
fn export_metadata_skips_non_audio_files() {
    let output = tag_cli()
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

    let output = tag_cli()
        .args([
            "export",
            "metadata",
            "-i",
            "*.mp3",
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
    let sidecar = out_dir.join("song.metadata.json");
    assert!(
        sidecar.exists(),
        "expected sidecar at {}",
        sidecar.display()
    );
}

#[test]
fn export_metadata_table_format() {
    let output = tag_cli()
        .args(["export", "metadata", "-i", "*.mp3", "-f", "table"])
        .output()
        .expect("failed to run tag-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Export summary"));
    assert!(stdout.contains("file_path"));
}
