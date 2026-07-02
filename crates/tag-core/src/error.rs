#[derive(Debug, thiserror::Error)]
pub enum TagCliError {
    #[error("failed to read manifest: {0}")]
    ManifestRead(String),
    #[error("TagLib error: {0}")]
    TagLib(#[from] crate::taglib::TagError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{command}: in-place modification requires -y/--yes (use --dry-run to preview)")]
    InPlaceNotConfirmed { command: String },
    #[error("{command}: writing file requires -y/--yes (use --dry-run to preview)")]
    WriteNotConfirmed { command: String },
    #[error("output path cannot be the same as input path")]
    SameInputOutput,
    #[error("unsupported tag key: {0}")]
    UnsupportedKey(String),
    #[error("unsupported image format: {0}")]
    UnsupportedImageFormat(String),
    #[error("failed to decode image: {0}")]
    ImageDecodeError(String),
    #[error("image processing error: {0}")]
    ImageProcessingError(String),
    #[error("apply failed: {0}")]
    ApplyFailed(String),
}
