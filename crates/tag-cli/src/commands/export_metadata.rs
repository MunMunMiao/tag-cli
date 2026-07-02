use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::ExportMetadataArgs;
use tag_core::config::expand_patterns;
use tag_core::error::TagCliError;
use tag_core::output::{
    ExportOrganization, ExportOutput, ExportRecord, ExportSummary, FailureRecord,
    OutputFormat as CoreOutputFormat, format_export,
};
use tag_core::taglib::{TagError, read_metadata_from_path_lenient};

enum OutputMode {
    Stdout,
    AggregateFile(PathBuf),
    SidecarDirectory(PathBuf),
}

pub fn run(args: &ExportMetadataArgs, verbose: bool) -> Result<(), TagCliError> {
    let base_dir = std::env::current_dir().map_err(TagCliError::Io)?;
    let files = expand_patterns(&args.input, &base_dir)?;

    if files.is_empty() {
        crate::report::status("Warning: no files matched the given patterns");
    }

    let mode = resolve_output_mode(args)?;
    let organization = resolve_organization(args);
    let format = resolve_format(args, &mode);

    let mut records = Vec::new();
    let mut failures = Vec::new();
    let mut succeeded = 0usize;
    let mut skipped = 0usize;

    for path in &files {
        if verbose {
            crate::report::status(format!("Reading {}", path.display()));
        }

        match read_one_file(path, &base_dir, args.absolute_paths) {
            Ok(Some(record)) => {
                records.push(record);
                succeeded += 1;
            }
            Ok(None) => {
                skipped += 1;
            }
            Err((category, message)) => {
                failures.push(FailureRecord {
                    file_path: path_to_string(path, &base_dir, args.absolute_paths),
                    read_status: "failed".to_string(),
                    error_category: category,
                    error_message: message,
                });
                if args.fail_fast {
                    break;
                }
            }
        }
    }

    apply_field_filter(
        &mut records,
        args.fields.as_deref(),
        args.exclude_fields.as_deref(),
    );

    let output = ExportOutput {
        export_timestamp: chrono::Utc::now().to_rfc3339(),
        generator: "tag-cli export metadata".to_string(),
        summary: ExportSummary {
            total: files.len(),
            succeeded,
            skipped,
            failed: failures.len(),
        },
        records,
        failures,
    };

    write_output(
        &output,
        format,
        organization,
        &mode,
        crate::cli::Cli::is_confirmed(args.yes),
    )?;

    crate::report::status(format!(
        "Success: {}, Skipped: {}, Failures: {}",
        output.summary.succeeded, output.summary.skipped, output.summary.failed
    ));

    if output.summary.failed > 0 {
        Err(TagCliError::ApplyFailed(format!(
            "{} file(s) failed to export",
            output.summary.failed
        )))
    } else {
        Ok(())
    }
}

fn resolve_output_mode(args: &ExportMetadataArgs) -> Result<OutputMode, TagCliError> {
    match &args.output {
        None => Ok(OutputMode::Stdout),
        Some(path) => {
            if args.per_file {
                ensure_directory(path, "sidecar output directory")?;
                Ok(OutputMode::SidecarDirectory(path.clone()))
            } else if args.aggregate {
                Ok(OutputMode::AggregateFile(path.clone()))
            } else if path.is_dir()
                || path.to_string_lossy().ends_with('/')
                || path.to_string_lossy().ends_with('\\')
            {
                ensure_directory(path, "sidecar output directory")?;
                Ok(OutputMode::SidecarDirectory(path.clone()))
            } else {
                Ok(OutputMode::AggregateFile(path.clone()))
            }
        }
    }
}

fn ensure_directory(path: &Path, description: &str) -> Result<(), TagCliError> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(TagCliError::Io)?;
    } else if !path.is_dir() {
        return Err(TagCliError::Io(std::io::Error::other(format!(
            "{description} is not a directory: {}",
            path.display()
        ))));
    }
    Ok(())
}

fn resolve_organization(args: &ExportMetadataArgs) -> ExportOrganization {
    if args.by_directory {
        ExportOrganization::ByDirectory
    } else if args.by_album {
        ExportOrganization::ByAlbum
    } else {
        ExportOrganization::Flat
    }
}

fn resolve_format(args: &ExportMetadataArgs, mode: &OutputMode) -> CoreOutputFormat {
    if let Some(f) = args.format {
        return match f {
            crate::cli::OutputFormat::Json => CoreOutputFormat::Json,
            crate::cli::OutputFormat::Yaml => CoreOutputFormat::Yaml,
            crate::cli::OutputFormat::Table => CoreOutputFormat::Table,
        };
    }

    match mode {
        OutputMode::AggregateFile(path) => match path.extension().and_then(|e| e.to_str()) {
            Some("json") => CoreOutputFormat::Json,
            Some("yaml") | Some("yml") => CoreOutputFormat::Yaml,
            _ => CoreOutputFormat::Json,
        },
        _ => CoreOutputFormat::Json,
    }
}

/// Heuristic list of audio file extensions that TagLib claims to support.
/// If one of these files is rejected by TagLib, we treat it as potentially
/// corrupt rather than an unsupported format.
const KNOWN_AUDIO_EXTENSIONS: [&str; 40] = [
    "mp3", "mp2", "m4a", "m4r", "m4b", "m4p", "mp4", "m4v", "3g2", "aac", "flac", "ogg", "opus",
    "oga", "spx", "wav", "aif", "aiff", "afc", "aifc", "wma", "asf", "ape", "mpc", "wv", "tta",
    "dsf", "dff", "dsdiff", "mod", "module", "nst", "wow", "s3m", "it", "xm", "shn", "mkv",
    "mka", "webm",
];

fn is_known_audio_format(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let ext = ext.to_lowercase();
            KNOWN_AUDIO_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

fn read_one_file(
    path: &Path,
    base_dir: &Path,
    absolute_paths: bool,
) -> Result<Option<ExportRecord>, (String, String)> {
    let metadata = match read_metadata_from_path_lenient(path) {
        Ok(m) => m,
        Err(TagError::InvalidFile) => {
            // Distinguish corrupt audio files from genuinely unsupported files
            // using a file-extension heuristic. TagLib's is_valid() returns 0
            // both for unknown formats and for truncated/corrupt known formats.
            if is_known_audio_format(path) {
                return Err((
                    "corrupt_file".to_string(),
                    "Could not read audio properties".to_string(),
                ));
            }
            crate::report::status(format!("Skipping {}: unsupported format", path.display()));
            return Ok(None); // unsupported format -> skip
        }
        Err(e) => {
            return Err((categorize_error(&e), e.to_string()));
        }
    };

    let file_path = path_to_string(path, base_dir, absolute_paths);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let relative_path = pathdiff::diff_paths(path, base_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.clone());
    let file_format = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    Ok(Some(ExportRecord::from_metadata(
        &metadata,
        &file_path,
        &file_name,
        &relative_path,
        &file_format,
    )))
}

fn categorize_error(error: &TagError) -> String {
    match error {
        TagError::InvalidPath => "invalid_path".to_string(),
        TagError::OpenFailed => "read_error".to_string(),
        TagError::InvalidFile => "unsupported_format".to_string(),
        TagError::MissingTag => "read_error".to_string(),
        TagError::SaveFailed => "read_error".to_string(),
        TagError::CoverSetFailed => "read_error".to_string(),
    }
}

fn path_to_string(path: &Path, base_dir: &Path, absolute_paths: bool) -> String {
    if absolute_paths {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string()
    } else {
        pathdiff::diff_paths(path, base_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
    }
}

fn apply_field_filter(
    records: &mut Vec<ExportRecord>,
    fields: Option<&str>,
    exclude_fields: Option<&str>,
) {
    let include: Vec<&str> = fields
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let exclude: Vec<&str> = exclude_fields
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();

    for record in records {
        // Apply exclusions first.
        for key in &exclude {
            match *key {
                "tags" => record.tags.clear(),
                "properties" => record.properties.clear(),
                "audio" => record.audio = None,
                "pictures" => {
                    record.pictures.count = 0;
                    record.pictures.front_cover_present = false;
                    record.pictures.summaries.clear();
                }
                "file_path" => record.file_path.clear(),
                "file_name" => record.file_name.clear(),
                "relative_path" => record.relative_path.clear(),
                "file_format" => record.file_format.clear(),
                "read_status" => record.read_status.clear(),
                "error_message" => record.error_message = None,
                _ if key.starts_with("tags.") => {
                    let tag_key = &key[5..];
                    record.tags.remove(tag_key);
                }
                _ => {}
            }
        }

        if !include.is_empty() {
            let allowed_toplevel: HashSet<&str> = include.iter().copied().collect();

            // Collect explicit tag keys requested via tags.<key>.
            let requested_tag_keys: HashSet<&str> = include
                .iter()
                .filter(|s| s.starts_with("tags."))
                .map(|s| &s[5..])
                .collect();

            if !allowed_toplevel.contains("tags") && requested_tag_keys.is_empty() {
                record.tags.clear();
            } else if !allowed_toplevel.contains("tags") && !requested_tag_keys.is_empty() {
                record
                    .tags
                    .retain(|k, _| requested_tag_keys.contains(k.as_str()));
            }

            if !allowed_toplevel.contains("properties") {
                record.properties.clear();
            }
            if !allowed_toplevel.contains("audio") {
                record.audio = None;
            }
            if !allowed_toplevel.contains("pictures") {
                record.pictures.count = 0;
                record.pictures.front_cover_present = false;
                record.pictures.summaries.clear();
            }
            if !allowed_toplevel.contains("file_path") {
                record.file_path.clear();
            }
            if !allowed_toplevel.contains("file_name") {
                record.file_name.clear();
            }
            if !allowed_toplevel.contains("relative_path") {
                record.relative_path.clear();
            }
            if !allowed_toplevel.contains("file_format") {
                record.file_format.clear();
            }
            if !allowed_toplevel.contains("read_status") {
                record.read_status.clear();
            }
            if !allowed_toplevel.contains("error_message") {
                record.error_message = None;
            }
        }
    }
}

fn write_output(
    output: &ExportOutput,
    format: CoreOutputFormat,
    organization: ExportOrganization,
    mode: &OutputMode,
    confirmed: bool,
) -> Result<(), TagCliError> {
    match mode {
        OutputMode::Stdout => {
            println!("{}", format_export(output, format, organization)?);
            Ok(())
        }
        OutputMode::AggregateFile(path) => {
            check_overwrite(path, confirmed)?;
            let content = format_export(output, format, organization)?;
            std::fs::write(path, content).map_err(TagCliError::Io)?;
            crate::report::status(format!("Wrote {}", path.display()));
            Ok(())
        }
        OutputMode::SidecarDirectory(dir) => {
            // Sidecar names are derived from the input file stem only, so files
            // with the same name from different source directories collide in the
            // flat output directory. Without -y the first existing file stops the
            // batch; with -y later files silently overwrite earlier ones.
            for record in &output.records {
                let stem = Path::new(&record.file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&record.file_name);
                let ext = match format {
                    CoreOutputFormat::Json => "json",
                    CoreOutputFormat::Yaml => "yaml",
                    CoreOutputFormat::Table => "txt",
                };
                let out_path = dir.join(format!("{}.metadata.{}", stem, ext));
                if out_path.exists() && !confirmed {
                    return Err(TagCliError::Io(std::io::Error::other(format!(
                        "output file already exists: {}. Use -y to overwrite.",
                        out_path.display()
                    ))));
                }

                let single = ExportOutput {
                    export_timestamp: output.export_timestamp.clone(),
                    generator: output.generator.clone(),
                    summary: ExportSummary {
                        total: 1,
                        succeeded: 1,
                        skipped: 0,
                        failed: 0,
                    },
                    records: vec![record.clone()],
                    failures: vec![],
                };
                let content = format_export(&single, format, ExportOrganization::Flat)?;
                std::fs::write(&out_path, content).map_err(TagCliError::Io)?;
            }
            crate::report::status(format!(
                "Wrote {} sidecar files to {}",
                output.records.len(),
                dir.display()
            ));
            Ok(())
        }
    }
}

fn check_overwrite(path: &Path, confirmed: bool) -> Result<(), TagCliError> {
    if path.exists() && !confirmed {
        return Err(TagCliError::Io(std::io::Error::other(format!(
            "output file already exists: {}. Use -y to overwrite.",
            path.display()
        ))));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tag_core::output::PicturesSummary;
    use tag_core::taglib::AudioProperties;

    fn sample_record() -> ExportRecord {
        ExportRecord {
            file_path: "./song.mp3".to_string(),
            file_name: "song.mp3".to_string(),
            relative_path: "./song.mp3".to_string(),
            file_format: "mp3".to_string(),
            tags: {
                let mut m = BTreeMap::new();
                m.insert("title".to_string(), "Song".to_string());
                m.insert("artist".to_string(), "Artist".to_string());
                m
            },
            properties: {
                let mut m = BTreeMap::new();
                m.insert("TITLE".to_string(), vec!["Song".to_string()]);
                m
            },
            audio: Some(AudioProperties {
                length_seconds: 120,
                bitrate_kbps: 320,
                sample_rate_hz: 44100,
                channels: 2,
            }),
            pictures: PicturesSummary {
                count: 1,
                front_cover_present: true,
                summaries: vec![],
            },
            read_status: "ok".to_string(),
            error_message: None,
        }
    }

    #[test]
    fn resolve_output_mode_stdout_when_no_output() {
        let args = ExportMetadataArgs {
            input: vec![],
            output: None,
            format: None,
            per_file: false,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert!(matches!(mode, OutputMode::Stdout));
    }

    #[test]
    fn resolve_output_mode_aggregate_for_file_path() {
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(PathBuf::from("report.json")),
            format: None,
            per_file: false,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert!(
            matches!(mode, OutputMode::AggregateFile(p) if p.as_path() == Path::new("report.json"))
        );
    }

    #[test]
    fn resolve_output_mode_sidecar_for_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(tmp.path().to_path_buf()),
            format: None,
            per_file: false,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert!(matches!(mode, OutputMode::SidecarDirectory(p) if p == tmp.path()));
    }

    #[test]
    fn resolve_output_mode_per_file_creates_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("new_sidecars");
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(out_dir.clone()),
            format: None,
            per_file: true,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert!(matches!(mode, OutputMode::SidecarDirectory(p) if p == out_dir));
        assert!(out_dir.exists());
    }

    #[test]
    fn resolve_format_defaults_to_json() {
        let args = ExportMetadataArgs {
            input: vec![],
            output: None,
            format: None,
            per_file: false,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        assert_eq!(
            resolve_format(&args, &OutputMode::Stdout),
            CoreOutputFormat::Json
        );
    }

    #[test]
    fn resolve_format_from_aggregate_file_extension() {
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(PathBuf::from("report.yaml")),
            format: None,
            per_file: false,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert_eq!(resolve_format(&args, &mode), CoreOutputFormat::Yaml);
    }

    #[test]
    fn resolve_format_explicit_overrides_extension() {
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(PathBuf::from("report.yaml")),
            format: Some(crate::cli::OutputFormat::Json),
            per_file: false,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert_eq!(resolve_format(&args, &mode), CoreOutputFormat::Json);
    }

    #[test]
    fn categorize_error_maps_invalid_path() {
        assert_eq!(
            categorize_error(&TagError::InvalidPath),
            "invalid_path".to_string()
        );
    }

    #[test]
    fn categorize_error_maps_invalid_file() {
        assert_eq!(
            categorize_error(&TagError::InvalidFile),
            "unsupported_format".to_string()
        );
    }

    #[test]
    fn path_to_string_relative_by_default() {
        let base = PathBuf::from("/tmp");
        let path = PathBuf::from("/tmp/music/song.mp3");
        assert_eq!(path_to_string(&path, &base, false), "music/song.mp3");
    }

    #[test]
    fn path_to_string_absolute_when_requested() {
        let base = PathBuf::from("/tmp");
        let path = PathBuf::from("/tmp/music/song.mp3");
        let result = path_to_string(&path, &base, true);
        assert!(result.ends_with("music/song.mp3"));
        assert!(result.starts_with('/'));
    }

    #[test]
    fn apply_field_filter_exclude_tags() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, None, Some("tags"));
        assert!(records[0].tags.is_empty());
        assert!(records[0].audio.is_some());
        assert_eq!(records[0].pictures.count, 1);
    }

    #[test]
    fn apply_field_filter_exclude_audio_and_pictures() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, None, Some("audio,pictures"));
        assert!(records[0].audio.is_none());
        assert_eq!(records[0].pictures.count, 0);
        assert!(!records[0].tags.is_empty());
    }

    #[test]
    fn apply_field_filter_include_only_tags() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("tags"), None);
        assert!(!records[0].tags.is_empty());
        assert!(records[0].properties.is_empty());
        assert!(records[0].audio.is_none());
        assert_eq!(records[0].pictures.count, 0);
    }

    #[test]
    fn apply_field_filter_include_tags_title_keeps_tags() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("tags.title"), None);
        assert!(!records[0].tags.is_empty());
    }

    #[test]
    fn apply_field_filter_include_specific_tag_keys() {
        let mut records = vec![sample_record()];
        apply_field_filter(
            &mut records,
            Some("file_path,tags.title,tags.artist,audio"),
            None,
        );
        assert!(!records[0].file_path.is_empty());
        assert_eq!(records[0].tags.len(), 2);
        assert!(records[0].tags.contains_key("title"));
        assert!(records[0].tags.contains_key("artist"));
        assert!(records[0].audio.is_some());
        assert!(records[0].properties.is_empty());
        assert_eq!(records[0].pictures.count, 0);
    }

    #[test]
    fn apply_field_filter_exclude_specific_tag_key() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, None, Some("tags.artist"));
        assert!(records[0].tags.contains_key("title"));
        assert!(!records[0].tags.contains_key("artist"));
        assert!(records[0].audio.is_some());
    }

    #[test]
    fn write_output_aggregate_file_requires_confirmation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("existing.json");
        std::fs::write(&path, "{}").unwrap();

        let output = ExportOutput {
            export_timestamp: "2026-07-02T10:00:00Z".to_string(),
            generator: "tag-cli export metadata".to_string(),
            summary: ExportSummary {
                total: 0,
                succeeded: 0,
                skipped: 0,
                failed: 0,
            },
            records: vec![],
            failures: vec![],
        };

        let result = write_output(
            &output,
            CoreOutputFormat::Json,
            ExportOrganization::Flat,
            &OutputMode::AggregateFile(path),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn write_output_aggregate_file_with_confirmation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("report.json");

        let output = ExportOutput {
            export_timestamp: "2026-07-02T10:00:00Z".to_string(),
            generator: "tag-cli export metadata".to_string(),
            summary: ExportSummary {
                total: 1,
                succeeded: 1,
                skipped: 0,
                failed: 0,
            },
            records: vec![sample_record()],
            failures: vec![],
        };

        write_output(
            &output,
            CoreOutputFormat::Json,
            ExportOrganization::Flat,
            &OutputMode::AggregateFile(path.clone()),
            true,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"total\": 1"));
        assert!(content.contains("\"title\": \"Song\""));
    }

    #[test]
    fn write_output_sidecar_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("sidecars");
        std::fs::create_dir(&dir).unwrap();

        let output = ExportOutput {
            export_timestamp: "2026-07-02T10:00:00Z".to_string(),
            generator: "tag-cli export metadata".to_string(),
            summary: ExportSummary {
                total: 1,
                succeeded: 1,
                skipped: 0,
                failed: 0,
            },
            records: vec![sample_record()],
            failures: vec![],
        };

        write_output(
            &output,
            CoreOutputFormat::Json,
            ExportOrganization::Flat,
            &OutputMode::SidecarDirectory(dir.clone()),
            true,
        )
        .unwrap();

        let sidecar = dir.join("song.metadata.json");
        assert!(sidecar.exists());
        let content = std::fs::read_to_string(&sidecar).unwrap();
        assert!(content.contains("\"title\": \"Song\""));
    }

    #[test]
    fn is_known_audio_format_recognizes_upstream_extensions() {
        let known = [
            "mp3", "mp2", "m4a", "m4r", "m4b", "m4p", "mp4", "m4v", "3g2", "aac", "flac", "ogg",
            "opus", "oga", "spx", "wav", "aif", "aiff", "afc", "aifc", "wma", "asf", "ape", "mpc",
            "wv", "tta", "dsf", "dff", "dsdiff", "mod", "module", "nst", "wow", "s3m", "it", "xm",
            "shn", "mkv", "mka", "webm",
        ];
        for ext in known {
            let path = PathBuf::from(format!("song.{ext}"));
            assert!(
                is_known_audio_format(&path),
                "expected {ext} to be a known audio format"
            );
        }

        // Case-insensitive matching.
        assert!(is_known_audio_format(Path::new("song.MP3")));
        assert!(is_known_audio_format(Path::new("song.FLAC")));

        // Unrelated extensions should not match.
        let unrelated = ["txt", "jpg", "png", "pdf", "doc", "exe", "zip", "xyz"];
        for ext in unrelated {
            let path = PathBuf::from(format!("song.{ext}"));
            assert!(
                !is_known_audio_format(&path),
                "expected {ext} to be an unknown format"
            );
        }
    }

    #[test]
    fn run_with_empty_patterns_reports_warning_and_succeeds() {
        let args = ExportMetadataArgs {
            input: vec![PathBuf::from("*.definitely_missing")],
            output: None,
            format: None,
            per_file: false,
            aggregate: false,
            flat: false,
            by_directory: false,
            by_album: false,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        assert!(run(&args, false).is_ok());
    }
}
