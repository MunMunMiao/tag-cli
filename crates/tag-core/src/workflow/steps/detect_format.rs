use std::path::Path;

use crate::error::TagCliError;
use crate::workflow::context::{AudioFormat, Context};
use crate::workflow::step::Step;

#[derive(Debug, Default)]
pub struct DetectAudioFormatStep;

impl DetectAudioFormatStep {
    pub fn new() -> Self {
        Self
    }
}

impl Step for DetectAudioFormatStep {
    fn name(&self) -> &'static str {
        "DetectAudioFormat"
    }

    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        let format = detect_format(&ctx.input_path);
        if ctx.verbose {
            tracing::info!("detected format: {:?}", format);
        }
        ctx.audio_format = Some(format);
        Ok(())
    }
}

fn detect_format(path: &Path) -> AudioFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => match ext.to_lowercase().as_str() {
            "mp3" | "mp2" => AudioFormat::Mpeg,
            "m4a" | "m4r" | "m4b" | "m4p" | "mp4" | "m4v" | "3g2" | "aac" => AudioFormat::Mp4,
            "flac" => AudioFormat::Flac,
            "ogg" => AudioFormat::OggVorbis,
            "opus" => AudioFormat::OggOpus,
            "oga" => AudioFormat::OggFlac,
            "spx" => AudioFormat::Speex,
            "wav" => AudioFormat::Wav,
            "aif" | "aiff" | "afc" | "aifc" => AudioFormat::Aiff,
            "wma" | "asf" => AudioFormat::Wma,
            "ape" => AudioFormat::Ape,
            "mpc" => AudioFormat::Mpc,
            "wv" => AudioFormat::WavPack,
            "tta" => AudioFormat::TrueAudio,
            "dsf" | "dff" | "dsdiff" => AudioFormat::Dsf,
            "mod" | "module" | "nst" | "wow" | "s3m" | "it" | "xm" => AudioFormat::Mod,
            "shn" => AudioFormat::Shorten,
            "mkv" | "mka" | "webm" => AudioFormat::Matroska,
            _ => AudioFormat::Other,
        },
        None => AudioFormat::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_formats() {
        assert_eq!(detect_format(Path::new("song.mp3")), AudioFormat::Mpeg);
        assert_eq!(detect_format(Path::new("song.MP3")), AudioFormat::Mpeg);
        assert_eq!(detect_format(Path::new("song.mp2")), AudioFormat::Mpeg);

        assert_eq!(detect_format(Path::new("song.m4a")), AudioFormat::Mp4);
        assert_eq!(detect_format(Path::new("song.m4r")), AudioFormat::Mp4);
        assert_eq!(detect_format(Path::new("song.m4b")), AudioFormat::Mp4);
        assert_eq!(detect_format(Path::new("song.m4p")), AudioFormat::Mp4);
        assert_eq!(detect_format(Path::new("song.mp4")), AudioFormat::Mp4);
        assert_eq!(detect_format(Path::new("song.m4v")), AudioFormat::Mp4);
        assert_eq!(detect_format(Path::new("song.3g2")), AudioFormat::Mp4);
        assert_eq!(detect_format(Path::new("song.aac")), AudioFormat::Mp4);

        assert_eq!(detect_format(Path::new("song.flac")), AudioFormat::Flac);

        assert_eq!(detect_format(Path::new("song.ogg")), AudioFormat::OggVorbis);
        assert_eq!(detect_format(Path::new("song.opus")), AudioFormat::OggOpus);
        assert_eq!(detect_format(Path::new("song.oga")), AudioFormat::OggFlac);
        assert_eq!(detect_format(Path::new("song.spx")), AudioFormat::Speex);

        assert_eq!(detect_format(Path::new("song.wav")), AudioFormat::Wav);

        assert_eq!(detect_format(Path::new("song.aiff")), AudioFormat::Aiff);
        assert_eq!(detect_format(Path::new("song.aif")), AudioFormat::Aiff);
        assert_eq!(detect_format(Path::new("song.afc")), AudioFormat::Aiff);
        assert_eq!(detect_format(Path::new("song.aifc")), AudioFormat::Aiff);

        assert_eq!(detect_format(Path::new("song.wma")), AudioFormat::Wma);
        assert_eq!(detect_format(Path::new("song.asf")), AudioFormat::Wma);

        assert_eq!(detect_format(Path::new("song.ape")), AudioFormat::Ape);
        assert_eq!(detect_format(Path::new("song.mpc")), AudioFormat::Mpc);
        assert_eq!(detect_format(Path::new("song.wv")), AudioFormat::WavPack);
        assert_eq!(detect_format(Path::new("song.tta")), AudioFormat::TrueAudio);

        assert_eq!(detect_format(Path::new("song.dsf")), AudioFormat::Dsf);
        assert_eq!(detect_format(Path::new("song.dff")), AudioFormat::Dsf);
        assert_eq!(detect_format(Path::new("song.dsdiff")), AudioFormat::Dsf);

        assert_eq!(detect_format(Path::new("song.mod")), AudioFormat::Mod);
        assert_eq!(detect_format(Path::new("song.module")), AudioFormat::Mod);
        assert_eq!(detect_format(Path::new("song.nst")), AudioFormat::Mod);
        assert_eq!(detect_format(Path::new("song.wow")), AudioFormat::Mod);
        assert_eq!(detect_format(Path::new("song.s3m")), AudioFormat::Mod);
        assert_eq!(detect_format(Path::new("song.it")), AudioFormat::Mod);
        assert_eq!(detect_format(Path::new("song.xm")), AudioFormat::Mod);

        assert_eq!(detect_format(Path::new("song.shn")), AudioFormat::Shorten);

        assert_eq!(detect_format(Path::new("song.mkv")), AudioFormat::Matroska);
        assert_eq!(detect_format(Path::new("song.mka")), AudioFormat::Matroska);
        assert_eq!(detect_format(Path::new("song.webm")), AudioFormat::Matroska);

        assert_eq!(detect_format(Path::new("song.xyz")), AudioFormat::Other);
        assert_eq!(detect_format(Path::new("no_extension")), AudioFormat::Other);
    }

    #[test]
    fn step_name_and_execute() {
        let step = DetectAudioFormatStep::new();
        assert_eq!(step.name(), "DetectAudioFormat");
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(step.execute(&mut ctx).is_ok());
        assert_eq!(ctx.audio_format, Some(AudioFormat::Mpeg));
    }

    #[test]
    fn verbose_logs_detected_format() {
        use crate::test_helpers::capture_logs;

        let step = DetectAudioFormatStep::new();
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("detected format: Mpeg"));
    }
}
