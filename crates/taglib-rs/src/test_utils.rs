use std::path::Path;
use std::process::Command;

pub fn generate_mp3(path: &Path) {
    generate_audio(path, &[]);
}

pub fn generate_flac(path: &Path) {
    generate_audio(path, &["-c:a", "flac"]);
}

fn generate_audio(path: &Path, codec_args: &[&str]) {
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("sine=frequency=1000:duration=1")
        .arg("-ar")
        .arg("44100")
        .arg("-ac")
        .arg("2")
        .arg("-metadata")
        .arg("TITLE=Original Title")
        .args(codec_args)
        .arg(path)
        .status()
        .expect("ffmpeg must be available");
    assert!(status.success());
}
