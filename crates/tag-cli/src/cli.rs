use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use tag_core::error::TagCliError;
use tag_core::workflow::context::{ImageProcessingConfig, ImageTargetFormat};

#[derive(Parser, Debug)]
#[command(name = "tag-cli")]
#[command(about = "A CLI tool for editing audio metadata via TagLib")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,
}

impl Cli {
    /// Determine whether a destructive write operation should be considered
    /// confirmed.
    ///
    /// Priority:
    /// 1. Explicit `-y`/`--yes` flag.
    /// 2. `TAG_CLI_YES=1` or `TAG_CLI_YES=true`.
    /// 3. `CI` set to any non-empty value other than `false`.
    pub fn is_confirmed(explicit_yes: bool) -> bool {
        if explicit_yes {
            return true;
        }
        if std::env::var("TAG_CLI_YES").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true")) {
            return true;
        }
        if std::env::var("CI").is_ok_and(|v| !v.is_empty() && !v.eq_ignore_ascii_case("false")) {
            return true;
        }
        false
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "List supported tag keys")]
    ListKeys(ListKeysArgs),

    #[command(about = "Show all metadata, audio properties, and embedded pictures")]
    Info(InfoArgs),

    #[command(about = "Read selected tag values")]
    Get(GetArgs),

    #[command(about = "Set tag values")]
    Set(SetArgs),

    #[command(about = "Clear selected or all tags")]
    Clear(ClearArgs),

    #[command(about = "Manage embedded cover art")]
    Cover(CoverArgs),

    #[command(about = "Apply a YAML manifest")]
    Apply(ApplyArgs),

    #[command(about = "Generate a minimal manifest template")]
    InitManifest(InitManifestArgs),

    #[command(subcommand)]
    Export(ExportCommands),

    #[command(about = "Generate shell completion script")]
    Completions(CompletionsArgs),

    #[command(about = "Generate man page")]
    Man,
}

#[derive(Parser, Debug)]
#[command(
    about = "Generate a minimal manifest template",
    long_about = "Generate a minimal manifest template file. This command creates or overwrites the output file and requires confirmation. Use --template to select a scenario-specific template.\n\nExamples:\n  tag-cli init-manifest -y\n  tag-cli init-manifest -y -o manifest.yaml\n  tag-cli init-manifest -y --template classical -o manifest.yaml"
)]
pub struct InitManifestArgs {
    #[arg(short = 'o', long, default_value = "manifest.yaml")]
    pub output: PathBuf,

    #[arg(
        short = 'y',
        long,
        help = "Skip confirmation prompt (also respects TAG_CLI_YES=1 or CI=true)"
    )]
    pub yes: bool,

    #[arg(
        long,
        value_enum,
        help = "Scenario template to use (classical, podcast, radio, education, vinyl, release)"
    )]
    pub template: Option<TemplateName>,
}

#[derive(Parser, Debug)]
#[command(
    about = "Generate shell completion script",
    long_about = "Generate a shell completion script for tag-cli and print it to stdout.\n\nExamples:\n  tag-cli completions bash > /etc/bash_completion.d/tag-cli\n  tag-cli completions zsh > /usr/local/share/zsh/site-functions/_tag-cli\n  tag-cli completions fish > ~/.config/fish/completions/tag-cli.fish"
)]
pub struct CompletionsArgs {
    #[arg(help = "Target shell (bash, zsh, fish)")]
    pub shell: Shell,
}

#[derive(Parser, Debug)]
#[command(
    about = "Show all metadata, audio properties, and embedded pictures",
    long_about = "Show all metadata, audio properties, and embedded pictures for an audio file.\n\nExamples:\n  tag-cli info -i song.mp3\n  tag-cli info -i song.mp3 -f json"
)]
pub struct InfoArgs {
    #[arg(short = 'i', long, help = "Input audio file path")]
    pub input: PathBuf,

    #[arg(short, long, value_enum, help = "Output format (json, yaml, table)")]
    pub format: Option<OutputFormat>,
}

#[derive(Parser, Debug)]
#[command(
    about = "List supported tag keys",
    long_about = "List the tag keys supported by tag-cli.\n\nExamples:\n  tag-cli list-keys\n  tag-cli list-keys --format json"
)]
pub struct ListKeysArgs {
    #[arg(short, long, value_enum, help = "Output format (json, yaml, table)")]
    pub format: Option<OutputFormat>,
}

#[derive(Parser, Debug)]
#[command(
    about = "Read selected tag values",
    long_about = "Read selected tag values from an audio file.\n\nExamples:\n  tag-cli get -i song.mp3 TITLE ARTIST\n  tag-cli get -i song.mp3 TITLE --format json"
)]
pub struct GetArgs {
    #[arg(short = 'i', long, help = "Input audio file path")]
    pub input: PathBuf,

    #[arg(help = "Tag keys to read")]
    pub keys: Vec<String>,

    #[arg(short, long, value_enum, help = "Output format (json, yaml, table)")]
    pub format: Option<OutputFormat>,
}

#[derive(Parser, Debug)]
#[command(
    about = "Set tag values",
    long_about = "Set tag values on an audio file. When no output path is given, the input file is modified in place and requires confirmation.\n\nExamples:\n  tag-cli set -i song.mp3 -y TITLE=\"My Title\" ARTIST=\"My Artist\"\n  tag-cli set -i song.mp3 -o output.mp3 -y TITLE=\"My Title\""
)]
pub struct SetArgs {
    #[arg(short = 'i', long, help = "Input audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        help = "Output audio file path (defaults to in-place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        short = 'y',
        long,
        help = "Skip confirmation prompt (also respects TAG_CLI_YES=1 or CI=true)"
    )]
    pub yes: bool,

    #[arg(long, help = "Show changes without applying them")]
    pub dry_run: bool,

    #[arg(
        long,
        short = 'R',
        help = "Replace all metadata: keep only the tags specified here and clear all other tag values."
    )]
    pub replace: bool,

    #[arg(value_parser = parse_key_value, help = "Tag key-value pairs as KEY=VALUE")]
    pub tags: Vec<(String, String)>,
}

#[derive(Parser, Debug)]
#[command(
    about = "Clear selected or all tags",
    long_about = "Clear selected tags or all supported tags and embedded cover art from an audio file. When no output path is given, the input file is modified in place and requires confirmation.\n\nExamples:\n  tag-cli clear -i song.mp3 -y --all\n  tag-cli clear -i song.mp3 -y TITLE ARTIST"
)]
pub struct ClearArgs {
    #[arg(short = 'i', long, help = "Input audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        help = "Output audio file path (defaults to in-place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        short = 'y',
        long,
        help = "Skip confirmation prompt (also respects TAG_CLI_YES=1 or CI=true)"
    )]
    pub yes: bool,

    #[arg(long, help = "Show changes without applying them")]
    pub dry_run: bool,

    #[arg(long, help = "Clear all supported tags and embedded cover art")]
    pub all: bool,

    #[arg(help = "Tag keys to clear")]
    pub keys: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(
    about = "Manage embedded cover art",
    long_about = "Manage embedded cover art.\n\nSubcommands:\n  get    Extract embedded cover art\n  set    Set embedded cover art from an image\n  clear  Remove embedded cover art\n\nExamples:\n  tag-cli cover get -i song.mp3 -o cover.jpg\n  tag-cli cover set -i song.mp3 -y cover.jpg\n  tag-cli cover clear -i song.mp3 -y"
)]
pub struct CoverArgs {
    #[command(subcommand)]
    pub command: CoverCommands,
}

#[derive(Subcommand, Debug)]
pub enum CoverCommands {
    #[command(
        about = "Extract embedded cover art",
        long_about = "Extract embedded cover art from an audio file to an image file.\n\nExamples:\n  tag-cli cover get -i song.mp3 -o cover.jpg"
    )]
    Get(CoverGetArgs),

    #[command(
        about = "Set embedded cover art from an image",
        long_about = "Set embedded cover art from an image file. When no output path is given, the input file is modified in place and requires confirmation.\n\nExamples:\n  tag-cli cover set -i song.mp3 -y cover.jpg\n  tag-cli cover set -i song.mp3 -y -o output.mp3 cover.jpg --cover-format jpeg --cover-quality 90"
    )]
    Set(CoverSetArgs),

    #[command(
        about = "Remove embedded cover art",
        long_about = "Remove embedded cover art from an audio file. When no output path is given, the input file is modified in place and requires confirmation.\n\nExamples:\n  tag-cli cover clear -i song.mp3 -y"
    )]
    Clear(CoverClearArgs),
}

#[derive(Parser, Debug)]
#[command(
    about = "Extract embedded cover art",
    long_about = "Extract embedded cover art from an audio file to an image file.\n\nExamples:\n  tag-cli cover get -i song.mp3 -o cover.jpg\n  tag-cli cover get -i song.mp3 -o back.jpg --picture-type \"Back Cover\""
)]
pub struct CoverGetArgs {
    #[arg(short = 'i', long, help = "Input audio file path")]
    pub input: PathBuf,

    #[arg(short = 'o', long, help = "Output image file path")]
    pub output: PathBuf,

    #[arg(
        long,
        value_name = "TYPE",
        help = "Picture type to extract (e.g. 'Back Cover')"
    )]
    pub picture_type: Option<String>,
}

#[derive(Parser, Debug)]
#[command(
    about = "Set embedded cover art from an image",
    long_about = "Set embedded cover art from an image file. When no output path is given, the input file is modified in place and requires confirmation.\n\nExamples:\n  tag-cli cover set -i song.mp3 -y cover.jpg\n  tag-cli cover set -i song.mp3 -y cover.jpg --cover-format jpeg --cover-quality 90\n  tag-cli cover set -i song.mp3 -y cover.jpg --picture-type \"Back Cover\""
)]
pub struct CoverSetArgs {
    #[arg(short = 'i', long, help = "Input audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        help = "Output audio file path (defaults to in-place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        short = 'y',
        long,
        help = "Skip confirmation prompt (also respects TAG_CLI_YES=1 or CI=true)"
    )]
    pub yes: bool,

    #[arg(long, help = "Show changes without applying them")]
    pub dry_run: bool,

    #[command(flatten)]
    pub image_options: ImageOptions,

    #[arg(
        long,
        value_name = "TYPE",
        help = "Picture type to set (e.g. 'Back Cover')"
    )]
    pub picture_type: Option<String>,

    #[arg(help = "Input image file path")]
    pub image: PathBuf,
}

#[derive(Parser, Debug)]
#[command(
    about = "Remove embedded cover art",
    long_about = "Remove embedded cover art from an audio file. When no output path is given, the input file is modified in place and requires confirmation.\n\nExamples:\n  tag-cli cover clear -i song.mp3 -y"
)]
pub struct CoverClearArgs {
    #[arg(short = 'i', long, help = "Input audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        help = "Output audio file path (defaults to in-place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        short = 'y',
        long,
        help = "Skip confirmation prompt (also respects TAG_CLI_YES=1 or CI=true)"
    )]
    pub yes: bool,

    #[arg(long, help = "Show changes without applying them")]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
#[command(
    about = "Apply a YAML manifest",
    long_about = "Apply a YAML manifest to one or more audio files.\n\nExamples:\n  tag-cli apply -m manifest.yaml -y\n  tag-cli apply -m manifest.yaml -y --dry-run --fail-fast"
)]
pub struct ApplyArgs {
    #[arg(
        short = 'm',
        long = "manifest",
        alias = "filename",
        visible_short_alias = 'f',
        value_name = "MANIFEST"
    )]
    pub filename: PathBuf,

    #[arg(
        short = 'y',
        long,
        help = "Skip confirmation prompt (also respects TAG_CLI_YES=1 or CI=true)"
    )]
    pub yes: bool,

    #[arg(long, help = "Show changes without applying them")]
    pub dry_run: bool,

    #[arg(long, help = "Stop on first failure")]
    pub fail_fast: bool,

    #[command(flatten)]
    pub image_options: ImageOptions,
}

#[derive(Parser, Debug, Clone)]
pub struct ImageOptions {
    #[arg(long, help = "Use the cover image as-is without reprocessing")]
    pub no_process_cover: bool,

    #[arg(long, value_enum, help = "Convert cover art to format (jpeg, png)")]
    pub cover_format: Option<CoverFormat>,

    #[arg(
        long,
        help = "Resize cover art so max width/height does not exceed this value"
    )]
    pub cover_max_size: Option<u32>,

    #[arg(
        long,
        help = "Compress cover art so file size does not exceed this value in KB"
    )]
    pub cover_max_file_size: Option<u32>,

    #[arg(long, help = "JPEG/PNG quality (1-100)")]
    pub cover_quality: Option<u8>,
}

impl ImageOptions {
    pub fn to_image_processing_config(&self) -> Result<ImageProcessingConfig, TagCliError> {
        let target_format = self.cover_format.map(|format| match format {
            CoverFormat::Jpeg => ImageTargetFormat::Jpeg,
            CoverFormat::Png => ImageTargetFormat::Png,
        });

        let quality = match self.cover_quality {
            None => 90,
            Some(q) => (q > 0 && q <= 100).then_some(q).ok_or_else(|| {
                TagCliError::ImageProcessingError(format!(
                    "cover quality must be between 1 and 100, got {q}"
                ))
            })?,
        };

        Ok(ImageProcessingConfig {
            no_process: self.no_process_cover,
            target_format,
            max_size: self.cover_max_size,
            max_file_size_kb: self.cover_max_file_size,
            quality,
            picture_type: None,
        })
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverFormat {
    Jpeg,
    Png,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateName {
    Classical,
    Podcast,
    Radio,
    Education,
    Vinyl,
    Release,
}

fn parse_key_value(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((k, v)) => Ok((k.to_uppercase(), v.to_string())),
        None => Err(format!("expected KEY=VALUE, got: {s}")),
    }
}

#[derive(Subcommand, Debug)]
pub enum ExportCommands {
    #[command(about = "Export metadata from audio files")]
    Metadata(ExportMetadataArgs),
}

#[derive(Parser, Debug)]
#[command(
    about = "Export metadata from audio files",
    long_about = "Export metadata and audio properties from audio files matched by glob patterns. Unsupported files are skipped. Output can be written to stdout, a single file, or per-file sidecars."
)]
pub struct ExportMetadataArgs {
    #[arg(
        short = 'i',
        long = "input",
        required = true,
        help = "Glob pattern or literal path; may be specified multiple times"
    )]
    pub input: Vec<PathBuf>,

    #[arg(
        short = 'o',
        long,
        help = "Output path: file -> aggregate report, directory -> per-file sidecars"
    )]
    pub output: Option<PathBuf>,

    #[arg(short, long, value_enum, help = "Output format (json, yaml, table)")]
    pub format: Option<OutputFormat>,

    #[arg(long, help = "Force per-file sidecar output even when -o is a file")]
    pub per_file: bool,

    #[arg(long, help = "Force aggregate report even when -o is a directory")]
    pub aggregate: bool,

    #[arg(
        long,
        group = "organization",
        help = "Output flat record array (default)"
    )]
    pub flat: bool,

    #[arg(
        long,
        group = "organization",
        help = "Group records by parent directory"
    )]
    pub by_directory: bool,

    #[arg(long, group = "organization", help = "Group records by album tag")]
    pub by_album: bool,

    #[arg(long, help = "Comma-separated field allowlist")]
    pub fields: Option<String>,

    #[arg(long, help = "Comma-separated field blocklist")]
    pub exclude_fields: Option<String>,

    #[arg(long, group = "path_style", help = "Use absolute paths in output")]
    pub absolute_paths: bool,

    #[arg(
        long,
        group = "path_style",
        help = "Use paths relative to the current directory (default)"
    )]
    pub relative_paths: bool,

    #[arg(long, help = "Stop on first unreadable file")]
    pub fail_fast: bool,

    #[arg(
        short = 'y',
        long,
        help = "Skip overwrite confirmation (also respects TAG_CLI_YES=1 or CI=true)"
    )]
    pub yes: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_options_jpeg_format() {
        let opts = ImageOptions {
            no_process_cover: false,
            cover_format: Some(CoverFormat::Jpeg),
            cover_max_size: None,
            cover_max_file_size: None,
            cover_quality: None,
        };
        let config = opts.to_image_processing_config().unwrap();
        assert_eq!(config.target_format, Some(ImageTargetFormat::Jpeg));
    }

    #[test]
    fn image_options_quality_zero_errors() {
        let opts = ImageOptions {
            no_process_cover: false,
            cover_format: None,
            cover_max_size: None,
            cover_max_file_size: None,
            cover_quality: Some(0),
        };
        assert!(opts.to_image_processing_config().is_err());
    }
}
