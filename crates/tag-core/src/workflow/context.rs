use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::taglib::{Metadata, Picture};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Mpeg,
    Mp4,
    Flac,
    OggVorbis,
    OggOpus,
    OggFlac,
    Speex,
    Wav,
    Aiff,
    Wma,
    Ape,
    Mpc,
    WavPack,
    TrueAudio,
    Dsf,
    Mod,
    Shorten,
    Matroska,
    Other,
}

#[derive(Debug, Clone, Default)]
pub struct TagUpdates {
    pub sets: BTreeMap<String, Vec<String>>,
    pub clears: Vec<String>,
    pub clear_all: bool,
    pub replace: bool,
}

#[derive(Debug, Clone, Default)]
pub enum CoverAction {
    #[default]
    Keep,
    Clear,
    Set(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageTargetFormat {
    Jpeg,
    Png,
}

#[derive(Debug, Clone)]
pub struct ImageProcessingConfig {
    pub no_process: bool,
    pub target_format: Option<ImageTargetFormat>,
    pub max_size: Option<u32>,
    pub max_file_size_kb: Option<u32>,
    pub quality: u8,
    pub picture_type: Option<String>,
}

impl Default for ImageProcessingConfig {
    fn default() -> Self {
        Self {
            no_process: false,
            target_format: None,
            max_size: None,
            max_file_size_kb: None,
            quality: 90,
            picture_type: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct StepReport {
    pub messages: Vec<String>,
}

#[derive(Debug)]
pub struct Context {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub dry_run: bool,
    pub verbose: bool,
    pub audio_format: Option<AudioFormat>,
    pub original_metadata: Option<Metadata>,
    pub metadata: Option<Metadata>,
    pub tag_updates: Option<TagUpdates>,
    pub cover_action: CoverAction,
    pub processed_cover: Option<Picture>,
    pub output: Option<String>,
    pub report: StepReport,
}

impl Context {
    pub fn new(input_path: impl Into<PathBuf>, dry_run: bool, verbose: bool) -> Self {
        Self {
            input_path: input_path.into(),
            output_path: None,
            dry_run,
            verbose,
            audio_format: None,
            original_metadata: None,
            metadata: None,
            tag_updates: None,
            cover_action: CoverAction::Keep,
            processed_cover: None,
            output: None,
            report: StepReport::default(),
        }
    }
}
