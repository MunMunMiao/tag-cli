use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use tag_core::error::TagCliError;
use tag_core::workflow::context::{ImageProcessingConfig, ImageTargetFormat};

#[derive(Parser, Debug)]
#[command(name = "tag-cli")]
#[command(
    about = "Edit audio metadata and embedded cover art",
    long_about = "Edit audio metadata and embedded cover art.

tag-cli wraps TagLib to read and write tags and cover images for MP3, FLAC, M4A, Ogg, Opus, WAV, and many other formats.

Common workflows:
  # Show everything about a file
  tag-cli info -i song.mp3

  # Read selected tags
  tag-cli get -i song.mp3 TITLE ARTIST

  # Preview a tag edit before writing
  tag-cli set -i song.mp3 --dry-run TITLE=\"My Title\"

  # Write tags in place after confirmation is explicit
  tag-cli set -i song.mp3 -y TITLE=\"My Title\" ARTIST=\"My Artist\"

  # Extract embedded cover art
  tag-cli cover get -i song.mp3 -o cover.jpg

  # Export metadata as an apply-ready YAML manifest
  tag-cli export metadata -i \"**/*.mp3\" -o manifest.yaml

Safety:
  Commands that modify files in place require -y/--yes.
  Use --dry-run first when a command supports it.",
    after_help = "Use \"tag-cli <COMMAND> --help\" for more information about a command.",
    help_template = "{before-help}{about-with-newline}
{usage-heading} {usage}

Inspect commands:
  info       Show all metadata, audio properties, and embedded pictures
  get        Read selected tag values
  list-keys  List supported tag keys

Edit commands:
  set        Set tag values
  clear      Clear selected or all tags
  cover      Manage embedded cover art

Batch commands:
  apply      Apply a YAML manifest
  export     Export metadata from audio files

Utility commands:
  update     Update tag-cli to the latest release

Options:
{options}
{after-help}"
)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        short,
        long,
        global = true,
        help = "Enable verbose/tracing output (debug logging is printed to stderr)"
    )]
    pub verbose: bool,
}

impl Cli {
    /// Return whether destructive writes were explicitly confirmed.
    pub fn is_confirmed(explicit_yes: bool) -> bool {
        explicit_yes
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(
        about = "List supported tag keys",
        long_about = "List the tag keys supported by tag-cli.

Examples:
  # List keys in table form
  tag-cli list-keys

  # List keys as JSON
  tag-cli list-keys --format json

  # List keys as YAML
  tag-cli list-keys --format yaml"
    )]
    ListKeys(ListKeysArgs),

    #[command(
        about = "Show all metadata, audio properties, and embedded pictures",
        long_about = "Show all metadata, audio properties, and embedded pictures for an audio file.

Examples:
  # Show all metadata in table form
  tag-cli info -i song.mp3

  # Show metadata as JSON
  tag-cli info -i song.mp3 -f json

  # Show metadata as YAML
  tag-cli info -i song.mp3 -f yaml"
    )]
    Info(InfoArgs),

    #[command(
        about = "Read selected tag values",
        long_about = "Read selected tag values from an audio file. When no keys are given, all tags are shown.

Examples:
  # Read selected tags
  tag-cli get -i song.mp3 TITLE ARTIST

  # Read every tag as JSON
  tag-cli get -i song.mp3 -f json

  # Read every tag as YAML
  tag-cli get -i song.mp3 -f yaml"
    )]
    Get(GetArgs),
    #[command(
        about = "Set tag values",
        long_about = "Set one or more tag values on an audio file.

When no output path is given, the input file is modified in place and requires confirmation. Use --dry-run to preview changes without writing, and -y to skip the confirmation prompt.

Examples:
  # Preview a tag edit before writing
  tag-cli set -i song.mp3 --dry-run TITLE=\"My Title\" ARTIST=\"My Artist\"

  # Write tags in place after confirmation is explicit
  tag-cli set -i song.mp3 -y TITLE=\"My Title\" ARTIST=\"My Artist\"

  # Write tags to a new output file
  tag-cli set -i song.mp3 -o output.mp3 TITLE=\"My Title\"

  # Replace all tags with the listed values
  tag-cli set -i song.mp3 -y --replace TITLE=\"My Title\" ARTIST=\"My Artist\""
    )]
    Set(SetArgs),

    #[command(
        about = "Clear selected or all tags",
        long_about = "Clear selected tags or all supported tags and embedded cover art from an audio file.

When no output path is given, the input file is modified in place and requires confirmation. Use --all to clear every supported tag and embedded cover; otherwise list the tags to clear.

Examples:
  # Preview clearing every supported tag and cover
  tag-cli clear -i song.mp3 --dry-run --all

  # Clear every supported tag and cover in place
  tag-cli clear -i song.mp3 -y --all

  # Clear selected tags in place
  tag-cli clear -i song.mp3 -y TITLE COMMENT"
    )]
    Clear(ClearArgs),

    #[command(
        about = "Manage embedded cover art",
        long_about = "Manage embedded cover art.

Available actions:
  get    Extract embedded cover art to an image file
  set    Set embedded cover art from an image file
  clear  Remove embedded cover art

Examples:
  # Extract embedded cover art
  tag-cli cover get -i song.mp3 -o cover.jpg

  # Set embedded cover art
  tag-cli cover set -i song.mp3 -y cover.jpg

  # Remove embedded cover art
  tag-cli cover clear -i song.mp3 -y"
    )]
    Cover(CoverArgs),
    #[command(
        about = "Apply a YAML manifest",
        long_about = "Apply a YAML manifest to one or more audio files.

The manifest declares target tags and covers for each file. Use --dry-run to preview every change, and -y to skip confirmation.

Examples:
  # Preview manifest changes before writing
  tag-cli apply -m manifest.yaml --dry-run

  # Apply manifest changes after confirmation is explicit
  tag-cli apply -m manifest.yaml -y

  # Stop on the first failed file
  tag-cli apply -m manifest.yaml -y --fail-fast"
    )]
    Apply(ApplyArgs),

    #[command(
        subcommand,
        about = "Export metadata from audio files",
        long_about = "Export metadata from audio files.

Examples:
  # Export metadata as an apply-ready YAML manifest
  tag-cli export metadata -i \"**/*.mp3\" -o manifest.yaml

  # Export metadata and embedded front cover images
  tag-cli export metadata -i \"**/*.mp3\" -o manifest.yaml --with-cover"
    )]
    Export(ExportCommands),
    #[command(
        about = "Update tag-cli to the latest release",
        long_about = "Update tag-cli to the latest release.

Checks GitHub Releases, downloads the matching binary for your platform, verifies the SHA256 checksum, and replaces the running executable. No confirmation prompt is shown.

Examples:
  # Update tag-cli in place
  tag-cli update"
    )]
    Update,
}

#[derive(Parser, Debug)]
pub struct InfoArgs {
    #[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
    pub input: PathBuf,

    #[arg(
        short,
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Output format (table, json, or yaml)"
    )]
    pub format: Option<OutputFormat>,
}

#[derive(Parser, Debug)]
pub struct ListKeysArgs {
    #[arg(
        short,
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Output format (table, json, or yaml)"
    )]
    pub format: Option<OutputFormat>,
}

#[derive(Parser, Debug)]
pub struct GetArgs {
    #[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
    pub input: PathBuf,

    #[arg(value_name = "KEY", help = "Tag keys to read (default: all tags)")]
    pub keys: Vec<String>,

    #[arg(
        short,
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Output format (table, json, or yaml)"
    )]
    pub format: Option<OutputFormat>,
}

#[derive(Parser, Debug)]
pub struct SetArgs {
    #[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        value_name = "FILE",
        help = "Output file path (default: edit input in place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(short = 'y', long, help = "Skip confirmation for destructive writes")]
    pub yes: bool,

    #[arg(long, help = "Preview changes and print a diff without writing")]
    pub dry_run: bool,

    #[arg(
        long,
        short = 'R',
        help = "Replace all metadata: keep only the listed tags and clear all other tag values"
    )]
    pub replace: bool,

    #[arg(
        value_parser = parse_key_value,
        value_name = "KEY=VALUE",
        help = "Tag assignment to write; repeat for multiple tags"
    )]
    pub tags: Vec<(String, String)>,
}

#[derive(Parser, Debug)]
pub struct ClearArgs {
    #[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        value_name = "FILE",
        help = "Output file path (default: edit input in place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(short = 'y', long, help = "Skip confirmation for destructive writes")]
    pub yes: bool,

    #[arg(long, help = "Preview changes and print a diff without writing")]
    pub dry_run: bool,

    #[arg(long, help = "Clear all supported tags and embedded cover art")]
    pub all: bool,

    #[arg(value_name = "KEY", help = "Tag keys to clear")]
    pub keys: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct CoverArgs {
    #[command(subcommand)]
    pub command: CoverCommands,
}

#[derive(Subcommand, Debug)]
pub enum CoverCommands {
    #[command(
        about = "Extract embedded cover art",
        long_about = "Extract embedded cover art from an audio file to an image file.

Examples:
  # Extract the front cover
  tag-cli cover get -i song.mp3 -o cover.jpg

  # Extract a different picture type
  tag-cli cover get -i song.mp3 -o back.jpg --picture-type \"Back Cover\""
    )]
    Get(CoverGetArgs),

    #[command(
        about = "Set embedded cover art from an image",
        long_about = "Set embedded cover art from an image file.

When no output path is given, the input file is modified in place and requires confirmation. Use --dry-run to preview changes without writing.

Examples:
  # Preview setting embedded cover art
  tag-cli cover set -i song.mp3 --dry-run cover.jpg

  # Set embedded cover art in place
  tag-cli cover set -i song.mp3 -y cover.jpg

  # Reprocess cover art while writing a new output file
  tag-cli cover set -i song.mp3 -y -o output.mp3 cover.jpg --cover-format jpeg --cover-quality 90

  # Set a different picture type
  tag-cli cover set -i song.mp3 -y cover.jpg --picture-type \"Back Cover\""
    )]
    Set(CoverSetArgs),

    #[command(
        about = "Remove embedded cover art",
        long_about = "Remove embedded cover art from an audio file.

When no output path is given, the input file is modified in place and requires confirmation. Use --dry-run to preview changes without writing.

Examples:
  # Preview removing embedded cover art
  tag-cli cover clear -i song.mp3 --dry-run

  # Remove embedded cover art in place
  tag-cli cover clear -i song.mp3 -y"
    )]
    Clear(CoverClearArgs),
}

#[derive(Parser, Debug)]
pub struct CoverGetArgs {
    #[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        value_name = "IMAGE",
        help = "Output image file path"
    )]
    pub output: PathBuf,

    #[arg(
        long,
        value_name = "TYPE",
        help = "Picture type to extract (example: Back Cover)"
    )]
    pub picture_type: Option<String>,
}

#[derive(Parser, Debug)]
pub struct CoverSetArgs {
    #[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        value_name = "FILE",
        help = "Output file path (default: edit input in place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(short = 'y', long, help = "Skip confirmation for destructive writes")]
    pub yes: bool,

    #[arg(long, help = "Preview changes and print a diff without writing")]
    pub dry_run: bool,

    #[command(flatten)]
    pub image_options: ImageOptions,

    #[arg(
        long,
        value_name = "TYPE",
        help = "Picture type to set (example: Back Cover)"
    )]
    pub picture_type: Option<String>,

    #[arg(value_name = "IMAGE", help = "Input image file path")]
    pub image: PathBuf,
}

#[derive(Parser, Debug)]
pub struct CoverClearArgs {
    #[arg(short = 'i', long, value_name = "FILE", help = "Audio file path")]
    pub input: PathBuf,

    #[arg(
        short = 'o',
        long,
        value_name = "FILE",
        help = "Output file path (default: edit input in place)"
    )]
    pub output: Option<PathBuf>,

    #[arg(short = 'y', long, help = "Skip confirmation for destructive writes")]
    pub yes: bool,

    #[arg(long, help = "Preview changes and print a diff without writing")]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct ApplyArgs {
    #[arg(
        short = 'm',
        long = "manifest",
        alias = "filename",
        visible_short_alias = 'f',
        value_name = "MANIFEST",
        help = "YAML manifest file path"
    )]
    pub filename: PathBuf,

    #[arg(short = 'y', long, help = "Skip confirmation for destructive writes")]
    pub yes: bool,

    #[arg(long, help = "Preview changes and print a diff without writing")]
    pub dry_run: bool,

    #[arg(long, help = "Stop on the first file failure")]
    pub fail_fast: bool,

    #[command(flatten)]
    pub image_options: ImageOptions,
}

#[derive(Parser, Debug, Clone)]
pub struct ImageOptions {
    #[arg(
        long,
        help = "Use the cover image as-is without reprocessing (default: reprocess cover art)"
    )]
    pub no_process_cover: bool,

    #[arg(
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Convert cover art to a specific format (jpeg or png)"
    )]
    pub cover_format: Option<CoverFormat>,

    #[arg(
        long,
        value_name = "PIXELS",
        help = "Resize cover art so max(width, height) <= PIXELS; defaults depend on target container"
    )]
    pub cover_max_size: Option<u32>,

    #[arg(
        long,
        value_name = "KB",
        help = "Compress cover art so file size <= KB; defaults depend on target container"
    )]
    pub cover_max_file_size: Option<u32>,

    #[arg(
        long,
        value_name = "QUALITY",
        help = "JPEG/PNG compression quality from 1 to 100 (default: 90)"
    )]
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

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
}

/// Map a CLI output format option to the core output format.
pub fn map_format(format: Option<OutputFormat>) -> tag_core::output::OutputFormat {
    match format {
        Some(OutputFormat::Json) => tag_core::output::OutputFormat::Json,
        Some(OutputFormat::Yaml) => tag_core::output::OutputFormat::Yaml,
        _ => tag_core::output::OutputFormat::Table,
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverFormat {
    Jpeg,
    Png,
}

fn parse_key_value(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((k, v)) => Ok((k.to_uppercase(), v.to_string())),
        None => Err(format!("expected KEY=VALUE, got: {s}")),
    }
}

#[derive(Subcommand, Debug)]
pub enum ExportCommands {
    #[command(
        about = "Export metadata as an apply-ready YAML manifest",
        long_about = "Export metadata from audio files as a YAML manifest that can be edited and applied back with `tag-cli apply`. Only YAML is supported.

Output modes:
  stdout (default)        Print the manifest to stdout.
  -o FILE                 Write the manifest to FILE.
  -o DIR  --per-file      Write one sidecar manifest per input into DIR.
  -o FILE --aggregate     Force aggregate manifest even if -o looks like a directory.

Cover extraction:
  --with-cover            Extract the front cover of each file to an external image file.
  --cover-dir DIR         Write cover images to DIR instead of the default location.
                          Only valid with --with-cover.

Field filtering:
  --fields FIELDS         Comma-separated allowlist (for example: TITLE,ARTIST,ALBUM).
  --exclude-fields FIELDS Comma-separated blocklist.

Path style:
  --relative-paths        Use paths relative to the current directory (default).
  --absolute-paths        Use absolute paths.

Multi-value tags are reduced to a single value (the first one) when written to the manifest.
Unsupported or corrupt files are skipped and reported at the end.

Examples:
  # Export a single file to stdout
  tag-cli export metadata -i song.mp3

  # Export all FLAC files to a manifest
  tag-cli export metadata -i \"**/*.flac\" -o manifest.yaml

  # Export a directory tree to per-file sidecars
  tag-cli export metadata -i \"music/**/*.mp3\" -o sidecars/ --per-file

  # Export with front cover images
  tag-cli export metadata -i \"**/*.mp3\" -o manifest.yaml --with-cover

  # Export only a few fields
  tag-cli export metadata -i \"**/*.mp3\" --fields TITLE,ARTIST,ALBUM

  # Stop on the first unreadable file
  tag-cli export metadata -i \"**/*.wav\" --fail-fast"
    )]
    Metadata(ExportMetadataArgs),
}

#[derive(Parser, Debug)]
pub struct ExportMetadataArgs {
    #[arg(
        short = 'i',
        long = "input",
        required = true,
        value_name = "PATTERN",
        help = "Input glob pattern or literal audio file path; repeat for multiple inputs"
    )]
    pub input: Vec<PathBuf>,

    #[arg(
        short = 'o',
        long,
        value_name = "PATH",
        help = "Output path: file for aggregate manifest, directory for per-file sidecars; stdout if omitted"
    )]
    pub output: Option<PathBuf>,

    #[arg(long, help = "Write one sidecar manifest per input file")]
    pub per_file: bool,

    #[arg(
        long,
        help = "Force aggregate manifest output even when -o is a directory path"
    )]
    pub aggregate: bool,

    #[arg(
        long,
        help = "Extract the front cover of each file to an external image file"
    )]
    pub with_cover: bool,

    #[arg(
        long,
        value_name = "DIR",
        requires = "with_cover",
        help = "Directory for extracted cover images (requires --with-cover)"
    )]
    pub cover_dir: Option<PathBuf>,

    #[arg(
        long,
        value_name = "FIELDS",
        help = "Comma-separated allowlist of fields to include (example: TITLE,ARTIST,ALBUM)"
    )]
    pub fields: Option<String>,

    #[arg(
        long,
        value_name = "FIELDS",
        help = "Comma-separated blocklist of fields to exclude"
    )]
    pub exclude_fields: Option<String>,

    #[arg(long, group = "path_style", help = "Use absolute paths in output")]
    pub absolute_paths: bool,

    #[arg(
        long,
        group = "path_style",
        help = "Use paths relative to the current directory (default)"
    )]
    pub relative_paths: bool,

    #[arg(
        long,
        help = "Stop on the first unreadable file instead of skipping it"
    )]
    pub fail_fast: bool,

    #[arg(short = 'y', long, help = "Skip confirmation for output overwrites")]
    pub yes: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_confirmed_true_with_explicit_yes() {
        assert!(Cli::is_confirmed(true));
    }

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
    fn image_options_png_format() {
        let opts = ImageOptions {
            no_process_cover: false,
            cover_format: Some(CoverFormat::Png),
            cover_max_size: None,
            cover_max_file_size: None,
            cover_quality: None,
        };
        let config = opts.to_image_processing_config().unwrap();
        assert_eq!(config.target_format, Some(ImageTargetFormat::Png));
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

    #[test]
    fn image_options_quality_over_100_errors() {
        let opts = ImageOptions {
            no_process_cover: false,
            cover_format: None,
            cover_max_size: None,
            cover_max_file_size: None,
            cover_quality: Some(101),
        };
        assert!(opts.to_image_processing_config().is_err());
    }

    #[test]
    fn parse_key_value_rejects_missing_equals() {
        assert!(parse_key_value("NOEQUALS").is_err());
    }

    #[test]
    fn cli_parses_version_flag() {
        let err = Cli::try_parse_from(["tag-cli", "--version"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn cli_parses_all_top_level_subcommands() {
        let _ = Cli::try_parse_from(["tag-cli", "list-keys"]).unwrap();
        let _ = Cli::try_parse_from(["tag-cli", "info", "-i", "song.mp3"]).unwrap();
        let _ = Cli::try_parse_from(["tag-cli", "get", "-i", "song.mp3", "TITLE"]).unwrap();
        let _ =
            Cli::try_parse_from(["tag-cli", "set", "-i", "song.mp3", "-y", "TITLE=Foo"]).unwrap();
        let _ = Cli::try_parse_from(["tag-cli", "clear", "-i", "song.mp3", "-y", "--all"]).unwrap();
        let _ = Cli::try_parse_from([
            "tag-cli",
            "cover",
            "get",
            "-i",
            "song.mp3",
            "-o",
            "cover.jpg",
        ])
        .unwrap();
        let _ = Cli::try_parse_from([
            "tag-cli",
            "cover",
            "set",
            "-i",
            "song.mp3",
            "-y",
            "cover.jpg",
        ])
        .unwrap();
        let _ = Cli::try_parse_from(["tag-cli", "cover", "clear", "-i", "song.mp3", "-y"]).unwrap();
        let _ = Cli::try_parse_from(["tag-cli", "apply", "-m", "manifest.yaml", "-y"]).unwrap();
        let _ = Cli::try_parse_from(["tag-cli", "export", "metadata", "-i", "song.mp3"]).unwrap();
        let _ = Cli::try_parse_from(["tag-cli", "update"]).unwrap();
    }

    #[test]
    fn cover_set_parses_image_options() {
        Cli::try_parse_from([
            "tag-cli",
            "cover",
            "set",
            "-i",
            "song.mp3",
            "-y",
            "--no-process-cover",
            "--cover-format",
            "png",
            "--cover-max-size",
            "500",
            "--cover-max-file-size",
            "200",
            "--cover-quality",
            "85",
            "cover.png",
        ])
        .unwrap();
    }

    #[test]
    fn apply_parses_image_options() {
        Cli::try_parse_from([
            "tag-cli",
            "apply",
            "-m",
            "manifest.yaml",
            "-y",
            "--no-process-cover",
            "--cover-format",
            "jpeg",
            "--cover-max-size",
            "800",
            "--cover-max-file-size",
            "300",
            "--cover-quality",
            "95",
        ])
        .unwrap();
    }
}
