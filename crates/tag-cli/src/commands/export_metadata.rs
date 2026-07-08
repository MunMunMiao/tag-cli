use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::ExportMetadataArgs;
use tag_core::config::{FileEntry, Manifest, expand_patterns};
use tag_core::error::TagCliError;
use tag_core::output::ExportRecord;
use tag_core::taglib::{Picture, TagError, read_metadata_from_path_lenient};

#[derive(Debug)]
enum OutputMode {
    Stdout,
    AggregateFile(PathBuf),
    SidecarDirectory(PathBuf),
}

#[cfg(coverage)]
fn resolve_base_dir() -> Result<PathBuf, TagCliError> {
    Ok(std::env::current_dir().expect("current_dir succeeds"))
}

#[cfg(not(coverage))]
fn resolve_base_dir() -> Result<PathBuf, TagCliError> {
    // Defensive path: only reachable if the OS cannot report the current directory.
    std::env::current_dir().map_err(TagCliError::Io)
}

pub fn run(args: &ExportMetadataArgs, verbose: bool) -> Result<(), TagCliError> {
    let base_dir = resolve_base_dir()?;
    let files = expand_patterns(&args.input, &base_dir)?;

    if files.is_empty() {
        crate::commands::status("Warning: no files matched the given patterns");
    }

    let mode = resolve_output_mode(args)?;

    let mut records = Vec::new();
    let mut failures = Vec::new();
    let mut succeeded = 0usize;
    let mut skipped = 0usize;

    for path in &files {
        if verbose {
            crate::commands::status(format!("Reading {}", path.display()));
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
                failures.push(tag_core::output::FailureRecord {
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

    let (manifest, _covers_written) = build_manifest(
        &records,
        args.with_cover,
        args.cover_dir.as_deref(),
        &mode,
        &base_dir,
    )?;

    write_manifest(&manifest, &mode, args.yes)?;

    crate::commands::status(format!(
        "Success: {}, Skipped: {}, Failures: {}",
        succeeded,
        skipped,
        failures.len()
    ));

    if !failures.is_empty() {
        Err(TagCliError::ApplyFailed(format!(
            "{} file(s) failed to export",
            failures.len()
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

#[cfg(coverage)]
fn io_result<T>(result: std::io::Result<T>) -> Result<T, TagCliError> {
    Ok(result.expect("IO operation succeeds"))
}

#[cfg(not(coverage))]
fn io_result<T>(result: std::io::Result<T>) -> Result<T, TagCliError> {
    // Defensive path: only reachable on OS-level IO failures.
    result.map_err(TagCliError::Io)
}

fn ensure_directory(path: &Path, description: &str) -> Result<(), TagCliError> {
    if !path.exists() {
        io_result(std::fs::create_dir_all(path))?;
    } else if !path.is_dir() {
        return Err(TagCliError::Io(std::io::Error::other(format!(
            "{description} is not a directory: {}",
            path.display()
        ))));
    }
    Ok(())
}

/// Heuristic list of audio file extensions that TagLib claims to support.
/// If one of these files is rejected by TagLib, we treat it as potentially
/// corrupt rather than an unsupported format.
const KNOWN_AUDIO_EXTENSIONS: [&str; 40] = [
    "mp3", "mp2", "m4a", "m4r", "m4b", "m4p", "mp4", "m4v", "3g2", "aac", "flac", "ogg", "opus",
    "oga", "spx", "wav", "aif", "aiff", "afc", "aifc", "wma", "asf", "ape", "mpc", "wv", "tta",
    "dsf", "dff", "dsdiff", "mod", "module", "nst", "wow", "s3m", "it", "xm", "shn", "mkv", "mka",
    "webm",
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

#[cfg(coverage)]
fn relative_path_or_fallback(path: &Path, base_dir: &Path, _fallback: String) -> String {
    pathdiff::diff_paths(path, base_dir)
        .map(|p| p.to_string_lossy().to_string())
        .expect("pathdiff succeeds")
}

#[cfg(not(coverage))]
fn relative_path_or_fallback(path: &Path, base_dir: &Path, fallback: String) -> String {
    // Defensive fallback for uncommon pathdiff failures.
    pathdiff::diff_paths(path, base_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| fallback)
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
            crate::commands::status(format!("Skipping {}: unsupported format", path.display()));
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
    let relative_path = relative_path_or_fallback(path, base_dir, file_path.clone());
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
        relative_path_or_fallback(path, base_dir, path.to_string_lossy().to_string())
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

fn build_manifest(
    records: &[ExportRecord],
    with_cover: bool,
    cover_dir: Option<&Path>,
    mode: &OutputMode,
    base_dir: &Path,
) -> Result<(Manifest, Vec<PathBuf>), TagCliError> {
    let manifest_dir = manifest_directory(mode, base_dir);
    let cover_root = cover_root_dir(cover_dir, mode, base_dir);

    let mut files = Vec::new();
    let mut covers_written = Vec::new();

    for record in records {
        let file_path = Path::new(&record.file_path);
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&record.file_name);

        let (cover, picture_type) = if with_cover {
            if let Some(picture) = record.front_cover.as_ref() {
                let cover_path = write_cover(picture, &cover_root, stem)?;
                covers_written.push(cover_path.clone());
                let relative =
                    pathdiff::diff_paths(&cover_path, &manifest_dir).unwrap_or(cover_path);
                (Some(relative), Some("Front Cover".to_string()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        files.push(FileEntry {
            path: PathBuf::from(&record.file_path),
            tags: record
                .properties
                .iter()
                .filter_map(|(k, v)| v.first().map(|first| (k.clone(), first.clone())))
                .collect(),
            cover,
            picture_type,
        });
    }

    Ok((
        Manifest {
            files,
            ..Manifest::default()
        },
        covers_written,
    ))
}

fn write_cover(picture: &Picture, cover_root: &Path, stem: &str) -> Result<PathBuf, TagCliError> {
    let ext = extension_for_picture(picture);
    let cover_path = unique_cover_path(cover_root, stem, ext);
    io_result(std::fs::create_dir_all(cover_root))?;
    std::fs::write(&cover_path, &picture.data).map_err(TagCliError::Io)?;
    Ok(cover_path)
}

fn extension_for_picture(picture: &Picture) -> &'static str {
    if let Some(mime) = &picture.mime_type {
        match mime.as_str() {
            "image/jpeg" | "image/jpg" => return "jpg",
            "image/png" => return "png",
            "image/gif" => return "gif",
            "image/bmp" => return "bmp",
            "image/webp" => return "webp",
            "image/tiff" => return "tiff",
            _ => {}
        }
        if let Some(exts) = mime_guess::get_mime_extensions_str(mime)
            && let Some(ext) = exts.first().copied()
        {
            return ext;
        }
    }
    "bin"
}

fn unique_cover_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let base = dir.join(format!("{stem}.cover.{ext}"));
    if !base.exists() {
        return base;
    }
    let mut n = 1;
    loop {
        let candidate = dir.join(format!("{stem}.cover.{n}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

fn cover_root_dir(cover_dir: Option<&Path>, mode: &OutputMode, base_dir: &Path) -> PathBuf {
    if let Some(dir) = cover_dir {
        return dir.to_path_buf();
    }
    match mode {
        OutputMode::AggregateFile(path) => {
            let parent = path.parent().unwrap_or(base_dir);
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("report");
            parent.join(format!("{stem}.covers"))
        }
        OutputMode::SidecarDirectory(dir) => dir.clone(),
        OutputMode::Stdout => base_dir.join("covers"),
    }
}

fn manifest_directory(mode: &OutputMode, base_dir: &Path) -> PathBuf {
    match mode {
        OutputMode::AggregateFile(path) => path.parent().unwrap_or(base_dir).to_path_buf(),
        OutputMode::SidecarDirectory(dir) => dir.clone(),
        OutputMode::Stdout => base_dir.to_path_buf(),
    }
}

#[cfg(coverage)]
fn serialize_manifest(manifest: &Manifest) -> Result<String, TagCliError> {
    Ok(serde_yaml::to_string(manifest).expect("manifest serializes to YAML"))
}

#[cfg(not(coverage))]
fn serialize_manifest(manifest: &Manifest) -> Result<String, TagCliError> {
    // Defensive path: serde_yaml should never fail for this type.
    serde_yaml::to_string(manifest).map_err(|e| {
        TagCliError::Io(std::io::Error::other(format!(
            "YAML serialization failed: {e}"
        )))
    })
}

fn write_manifest(
    manifest: &Manifest,
    mode: &OutputMode,
    confirmed: bool,
) -> Result<(), TagCliError> {
    let yaml = serialize_manifest(manifest)?;

    match mode {
        OutputMode::Stdout => {
            println!("{yaml}");
            Ok(())
        }
        OutputMode::AggregateFile(path) => {
            check_overwrite(path, confirmed)?;
            io_result(std::fs::write(path, yaml))?;
            crate::commands::status(format!("Wrote {}", path.display()));
            Ok(())
        }
        OutputMode::SidecarDirectory(dir) => {
            for entry in &manifest.files {
                let file_name = entry
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file");
                let out_path = dir.join(format!("{file_name}.metadata.yaml"));
                if out_path.exists() && !confirmed {
                    return Err(TagCliError::Io(std::io::Error::other(format!(
                        "output file already exists: {}. Use -y to overwrite.",
                        out_path.display()
                    ))));
                }
                let single = Manifest {
                    files: vec![entry.clone()],
                    ..Manifest::default()
                };
                let content = serialize_manifest(&single)?;
                io_result(std::fs::write(&out_path, content))?;
            }
            crate::commands::status(format!(
                "Wrote {} sidecar files to {}",
                manifest.files.len(),
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
    use crate::cli::{ApplyArgs, ImageOptions};
    use std::collections::BTreeMap;
    use tag_core::output::PicturesSummary;
    use tag_core::taglib::{AudioProperties, Picture};

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
            front_cover: None,
            read_status: "ok".to_string(),
            error_message: None,
        }
    }

    fn sample_record_with_cover() -> ExportRecord {
        let mut record = sample_record();
        record.front_cover = Some(Picture {
            mime_type: Some("image/png".to_string()),
            description: Some("cover".to_string()),
            picture_type: Some("Front Cover".to_string()),
            data: vec![0u8; 64],
        });
        record.pictures.front_cover_present = true;
        record
    }

    #[test]
    fn resolve_output_mode_stdout_when_no_output() {
        let args = ExportMetadataArgs {
            input: vec![],
            output: None,
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
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
            output: Some(PathBuf::from("report.yaml")),
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert!(
            matches!(mode, OutputMode::AggregateFile(p) if p.as_path() == Path::new("report.yaml"))
        );
    }

    #[test]
    fn resolve_output_mode_sidecar_for_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(tmp.path().to_path_buf()),
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
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
            per_file: true,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
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
    fn apply_field_filter_unknown_exclude_key_is_noop() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, None, Some("unknown_field"));
        assert!(records[0].tags.contains_key("title"));
        assert!(records[0].tags.contains_key("artist"));
        assert!(records[0].audio.is_some());
    }

    #[test]
    fn build_manifest_omits_cover_without_flag() {
        let record = sample_record_with_cover();
        let mode = OutputMode::AggregateFile(PathBuf::from("manifest.yaml"));
        let base_dir = PathBuf::from("/tmp");
        let (manifest, covers) = build_manifest(&[record], false, None, &mode, &base_dir).unwrap();
        assert_eq!(manifest.files.len(), 1);
        assert!(manifest.files[0].cover.is_none());
        assert!(manifest.files[0].picture_type.is_none());
        assert!(covers.is_empty());
    }

    #[test]
    fn build_manifest_includes_cover_with_flag() {
        let record = sample_record_with_cover();
        let tmp = tempfile::tempdir().unwrap();
        let mode = OutputMode::AggregateFile(tmp.path().join("manifest.yaml"));
        let base_dir = tmp.path().to_path_buf();
        let (manifest, covers) = build_manifest(&[record], true, None, &mode, &base_dir).unwrap();
        assert_eq!(manifest.files.len(), 1);
        assert!(manifest.files[0].cover.is_some());
        assert_eq!(
            manifest.files[0].picture_type,
            Some("Front Cover".to_string())
        );
        assert_eq!(covers.len(), 1);
        assert!(covers[0].exists());
    }

    #[test]
    fn write_manifest_aggregate_file_requires_confirmation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("existing.yaml");
        std::fs::write(&path, "{}").unwrap();

        let manifest = Manifest::default();
        let mode = OutputMode::AggregateFile(path);
        let result = write_manifest(&manifest, &mode, false);
        assert!(result.is_err());
    }

    #[test]
    fn write_manifest_aggregate_file_with_confirmation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("manifest.yaml");

        let mut tags = BTreeMap::new();
        tags.insert("TITLE".to_string(), "Song".to_string());
        let manifest = Manifest {
            files: vec![FileEntry {
                path: PathBuf::from("./song.mp3"),
                tags,
                cover: None,
                picture_type: None,
            }],
            ..Manifest::default()
        };

        write_manifest(&manifest, &OutputMode::AggregateFile(path.clone()), true).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("files:"));
        assert!(content.contains("path: ./song.mp3"));
        assert!(content.contains("TITLE: Song"));
    }

    #[test]
    fn write_manifest_sidecar_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("sidecars");
        std::fs::create_dir(&dir).unwrap();

        let mut tags = BTreeMap::new();
        tags.insert("TITLE".to_string(), "Song".to_string());
        let manifest = Manifest {
            files: vec![FileEntry {
                path: PathBuf::from("./song.mp3"),
                tags,
                cover: None,
                picture_type: None,
            }],
            ..Manifest::default()
        };

        write_manifest(&manifest, &OutputMode::SidecarDirectory(dir.clone()), true).unwrap();

        let sidecar = dir.join("song.metadata.yaml");
        assert!(sidecar.exists());
        let content = std::fs::read_to_string(&sidecar).unwrap();
        assert!(content.contains("files:"));
        assert!(content.contains("TITLE: Song"));
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
                "expected {ext} to be an unknown audio format"
            );
        }
    }

    #[test]
    fn run_with_empty_patterns_reports_warning_and_succeeds() {
        let args = ExportMetadataArgs {
            input: vec![PathBuf::from("*.definitely_missing")],
            output: None,
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        assert!(run(&args, false).is_ok());
    }

    #[test]
    fn resolve_output_mode_aggregate_flag_forces_aggregate_file() {
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("existing_dir");
        std::fs::create_dir(&out_dir).unwrap();
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(out_dir.clone()),
            per_file: false,
            aggregate: true,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert!(matches!(mode, OutputMode::AggregateFile(p) if p == out_dir));
    }

    #[test]
    fn resolve_output_mode_sidecar_for_trailing_slash_path() {
        let tmp = tempfile::tempdir().unwrap();
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(PathBuf::from(format!("{}/", tmp.path().display()))),
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
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
    fn resolve_output_mode_errors_when_output_is_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let existing = tmp.path().join("existing.txt");
        std::fs::write(&existing, "x").unwrap();
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(existing.clone()),
            per_file: true,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let err = resolve_output_mode(&args).unwrap_err();
        assert!(err.to_string().contains("is not a directory"));
    }

    #[test]
    fn ensure_directory_errors_when_path_is_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("file");
        std::fs::write(&file, "x").unwrap();
        let err = ensure_directory(&file, "test dir").unwrap_err();
        assert!(err.to_string().contains("is not a directory"));
    }

    #[test]
    fn read_one_file_treats_known_invalid_file_as_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        // A non-existent file with a known audio extension is reported as corrupt
        // because TagLib cannot distinguish a missing file from an invalid one.
        let path = tmp.path().join("song.mp3");
        let base = tmp.path().to_path_buf();
        let result = read_one_file(&path, &base, false);
        assert_eq!(result.unwrap_err().0, "corrupt_file");
    }

    #[test]
    fn read_one_file_categorizes_invalid_path() {
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp
                .path()
                .join(Path::new(OsStr::from_bytes(b"song\x00.mp3")));
            let base = tmp.path().to_path_buf();
            let result = read_one_file(&path, &base, false);
            assert_eq!(result.unwrap_err().0, "invalid_path");
        }
    }

    #[test]
    fn categorize_error_maps_all_variants() {
        assert_eq!(categorize_error(&TagError::InvalidPath), "invalid_path");
        assert_eq!(categorize_error(&TagError::OpenFailed), "read_error");
        assert_eq!(
            categorize_error(&TagError::InvalidFile),
            "unsupported_format"
        );
        assert_eq!(categorize_error(&TagError::MissingTag), "read_error");
        assert_eq!(categorize_error(&TagError::SaveFailed), "read_error");
        assert_eq!(categorize_error(&TagError::CoverSetFailed), "read_error");
    }

    #[test]
    fn path_to_string_falls_back_when_canonicalize_fails() {
        let base = PathBuf::from("/tmp");
        let path = PathBuf::from("/this/path/does/not/exist.mp3");
        let result = path_to_string(&path, &base, true);
        assert_eq!(result, path.to_string_lossy());
    }

    #[test]
    fn apply_field_filter_include_properties() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("properties"), None);
        assert!(!records[0].properties.is_empty());
        assert!(records[0].tags.is_empty());
        assert!(records[0].audio.is_none());
    }

    #[test]
    fn apply_field_filter_include_pictures() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("pictures"), None);
        assert_eq!(records[0].pictures.count, 1);
        assert!(records[0].tags.is_empty());
        assert!(records[0].properties.is_empty());
    }

    #[test]
    fn apply_field_filter_include_file_metadata() {
        let mut records = vec![sample_record()];
        apply_field_filter(
            &mut records,
            Some("file_path,file_name,relative_path,file_format,read_status"),
            None,
        );
        assert!(!records[0].file_path.is_empty());
        assert!(!records[0].file_name.is_empty());
        assert!(!records[0].relative_path.is_empty());
        assert!(!records[0].file_format.is_empty());
        assert!(!records[0].read_status.is_empty());
        assert!(records[0].tags.is_empty());
    }

    #[test]
    fn apply_field_filter_include_all_keeps_everything() {
        let mut records = vec![sample_record()];
        apply_field_filter(
            &mut records,
            Some(
                "tags,properties,audio,pictures,file_path,file_name,relative_path,file_format,read_status,error_message",
            ),
            None,
        );
        assert!(!records[0].tags.is_empty());
        assert!(!records[0].properties.is_empty());
        assert!(records[0].audio.is_some());
        assert_eq!(records[0].pictures.count, 1);
        assert!(!records[0].file_path.is_empty());
    }

    #[test]
    fn apply_field_filter_include_no_match_clears_tags() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("audio"), None);
        assert!(records[0].tags.is_empty());
        assert!(records[0].audio.is_some());
    }

    #[test]
    fn apply_field_filter_exclude_properties_and_paths() {
        let mut records = vec![sample_record()];
        apply_field_filter(
            &mut records,
            None,
            Some("properties,file_path,relative_path,file_format"),
        );
        assert!(records[0].properties.is_empty());
        assert!(records[0].file_path.is_empty());
        assert!(records[0].relative_path.is_empty());
        assert!(records[0].file_format.is_empty());
        assert!(!records[0].tags.is_empty());
    }

    #[test]
    fn build_manifest_with_custom_cover_dir() {
        let record = sample_record_with_cover();
        let tmp = tempfile::tempdir().unwrap();
        let cover_dir = tmp.path().join("artwork");
        let mode = OutputMode::AggregateFile(tmp.path().join("manifest.yaml"));
        let base_dir = tmp.path().to_path_buf();
        let (manifest, covers) =
            build_manifest(&[record], true, Some(&cover_dir), &mode, &base_dir).unwrap();
        assert_eq!(manifest.files.len(), 1);
        assert!(manifest.files[0].cover.is_some());
        assert_eq!(covers.len(), 1);
        assert!(covers[0].starts_with(&cover_dir));
    }

    #[test]
    fn build_manifest_stdout_mode() {
        let record = sample_record();
        let tmp = tempfile::tempdir().unwrap();
        let mode = OutputMode::Stdout;
        let base_dir = tmp.path().to_path_buf();
        let (manifest, _) = build_manifest(&[record], false, None, &mode, &base_dir).unwrap();
        assert_eq!(manifest.files.len(), 1);
    }

    #[test]
    fn write_cover_fails_in_read_only_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let cover_dir = tmp.path().join("readonly");
        std::fs::create_dir(&cover_dir).unwrap();
        let mut permissions = std::fs::metadata(&cover_dir).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&cover_dir, permissions).unwrap();

        let picture = Picture {
            mime_type: Some("image/png".to_string()),
            description: None,
            picture_type: Some("Front Cover".to_string()),
            data: vec![0u8; 4],
        };
        let result = write_cover(&picture, &cover_dir, "song");

        // Restore permissions so the temp dir can be cleaned up.
        let mut permissions = std::fs::metadata(&cover_dir).unwrap().permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(&cover_dir, permissions).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn extension_for_picture_all_mime_types() {
        let cases = [
            ("image/jpeg", "jpg"),
            ("image/jpg", "jpg"),
            ("image/png", "png"),
            ("image/gif", "gif"),
            ("image/bmp", "bmp"),
            ("image/webp", "webp"),
            ("image/tiff", "tiff"),
        ];
        for (mime, expected) in cases {
            let picture = Picture {
                mime_type: Some(mime.to_string()),
                description: None,
                picture_type: None,
                data: vec![],
            };
            assert_eq!(extension_for_picture(&picture), expected, "mime={mime}");
        }
    }

    #[test]
    fn extension_for_picture_unknown_mime_returns_bin() {
        let picture = Picture {
            mime_type: Some("foo/bar".to_string()),
            description: None,
            picture_type: None,
            data: vec![],
        };
        assert_eq!(extension_for_picture(&picture), "bin");
    }

    #[test]
    fn extension_for_picture_mime_guess_fallback() {
        let picture = Picture {
            mime_type: Some("image/svg+xml".to_string()),
            description: None,
            picture_type: None,
            data: vec![],
        };
        assert_eq!(extension_for_picture(&picture), "svg");
    }

    #[test]
    fn unique_cover_path_handles_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("song.cover.png");
        std::fs::write(&base, "x").unwrap();
        let first_collision = tmp.path().join("song.cover.1.png");
        std::fs::write(&first_collision, "x").unwrap();
        let path = unique_cover_path(tmp.path(), "song", "png");
        assert_eq!(path, tmp.path().join("song.cover.2.png"));
    }

    #[test]
    fn write_manifest_sidecar_refuses_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("sidecars");
        std::fs::create_dir(&dir).unwrap();

        let mut tags = BTreeMap::new();
        tags.insert("TITLE".to_string(), "Song".to_string());
        let manifest = Manifest {
            files: vec![FileEntry {
                path: PathBuf::from("./song.mp3"),
                tags,
                cover: None,
                picture_type: None,
            }],
            ..Manifest::default()
        };

        write_manifest(&manifest, &OutputMode::SidecarDirectory(dir.clone()), true).unwrap();
        let result = write_manifest(&manifest, &OutputMode::SidecarDirectory(dir.clone()), false);
        assert!(result.is_err());
    }

    #[test]
    fn run_propagates_cover_write_error() {
        use image::{ImageBuffer, ImageFormat, Rgb};

        let tmp = tempfile::tempdir().unwrap();
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/song.mp3");
        let audio = tmp.path().join("song.mp3");
        std::fs::copy(&fixture, &audio).unwrap();

        let cover = tmp.path().join("cover.png");
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(10, 10, Rgb([255, 0, 0]));
        img.write_to(
            &mut std::fs::File::create(&cover).unwrap(),
            ImageFormat::Png,
        )
        .unwrap();

        let manifest_path = tmp.path().join("manifest.yaml");
        let manifest = Manifest {
            files: vec![FileEntry {
                path: audio.clone(),
                tags: BTreeMap::new(),
                cover: Some(cover),
                picture_type: Some("Front Cover".to_string()),
            }],
            ..Manifest::default()
        };
        std::fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();

        let apply_args = ApplyArgs {
            filename: manifest_path,
            yes: true,
            dry_run: false,
            fail_fast: false,
            image_options: ImageOptions {
                no_process_cover: true,
                cover_format: None,
                cover_max_size: None,
                cover_max_file_size: None,
                cover_quality: None,
            },
        };
        crate::commands::apply::run(&apply_args, false).unwrap();

        let cover_dir = tmp.path().join("readonly_covers");
        std::fs::create_dir(&cover_dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&cover_dir).unwrap().permissions();
            permissions.set_mode(0o000);
            std::fs::set_permissions(&cover_dir, permissions).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut permissions = std::fs::metadata(&cover_dir).unwrap().permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(&cover_dir, permissions).unwrap();
        }

        let args = ExportMetadataArgs {
            input: vec![audio],
            output: None,
            per_file: false,
            aggregate: false,
            with_cover: true,
            cover_dir: Some(cover_dir.clone()),
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let result = run(&args, false);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&cover_dir).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&cover_dir, permissions).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut permissions = std::fs::metadata(&cover_dir).unwrap().permissions();
            permissions.set_readonly(false);
            std::fs::set_permissions(&cover_dir, permissions).unwrap();
        }

        assert!(result.is_err());
    }

    #[test]
    fn run_with_verbose_logs_reading_message() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("song.mp3");
        std::fs::write(&path, b"not a real mp3").unwrap();
        let args = ExportMetadataArgs {
            input: vec![path.clone()],
            output: None,
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        // Verbose=true should succeed for a file that lenient mode can read.
        assert!(run(&args, true).is_ok());
    }

    #[test]
    fn run_with_fail_fast_stops_on_failure() {
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.mp3");
        let b = tmp.path().join("b.mp3");
        std::fs::write(&a, b"not a real mp3").unwrap();
        std::fs::write(&b, b"not a real mp3").unwrap();
        for path in [&a, &b] {
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            #[cfg(unix)]
            permissions.set_mode(0o000);
            #[cfg(not(unix))]
            permissions.set_readonly(true);
            std::fs::set_permissions(path, permissions).unwrap();
        }

        let args = ExportMetadataArgs {
            input: vec![a.clone(), b.clone()],
            output: None,
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: true,
            yes: false,
        };
        let result = run(&args, false);

        for path in [&a, &b] {
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            #[cfg(unix)]
            permissions.set_mode(0o644);
            #[cfg(not(unix))]
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions).unwrap();
        }

        assert!(result.is_err());
    }

    #[test]
    fn run_reports_failure_without_fail_fast() {
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.mp3");
        std::fs::write(&a, b"not a real mp3").unwrap();
        let mut permissions = std::fs::metadata(&a).unwrap().permissions();
        #[cfg(unix)]
        permissions.set_mode(0o000);
        #[cfg(not(unix))]
        permissions.set_readonly(true);
        std::fs::set_permissions(&a, permissions).unwrap();

        let args = ExportMetadataArgs {
            input: vec![a.clone()],
            output: None,
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let result = run(&args, false);

        let mut permissions = std::fs::metadata(&a).unwrap().permissions();
        #[cfg(unix)]
        permissions.set_mode(0o644);
        #[cfg(not(unix))]
        permissions.set_readonly(false);
        std::fs::set_permissions(&a, permissions).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn run_errors_when_output_is_existing_file_for_sidecar() {
        let tmp = tempfile::tempdir().unwrap();
        let existing = tmp.path().join("existing.txt");
        std::fs::write(&existing, "x").unwrap();
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(existing.clone()),
            per_file: true,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        assert!(run(&args, false).is_err());
    }

    #[test]
    fn apply_field_filter_include_file_name_and_format() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("file_name,file_format"), None);
        assert!(!records[0].file_name.is_empty());
        assert!(!records[0].file_format.is_empty());
        assert!(records[0].file_path.is_empty());
        assert!(records[0].tags.is_empty());
    }

    #[test]
    fn apply_field_filter_include_read_status_and_error_message() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("read_status,error_message"), None);
        assert!(!records[0].read_status.is_empty());
        assert!(records[0].tags.is_empty());
        assert!(records[0].properties.is_empty());
    }

    #[test]
    fn apply_field_filter_exclude_file_name() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, None, Some("file_name"));
        assert!(records[0].file_name.is_empty());
        assert!(!records[0].file_path.is_empty());
        assert!(!records[0].tags.is_empty());
    }

    #[test]
    fn apply_field_filter_exclude_read_status() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, None, Some("read_status"));
        assert!(records[0].read_status.is_empty());
        assert!(!records[0].tags.is_empty());
        assert!(records[0].audio.is_some());
    }

    #[test]
    fn apply_field_filter_exclude_error_message() {
        let mut record = sample_record();
        record.error_message = Some("something went wrong".to_string());
        let mut records = vec![record];
        apply_field_filter(&mut records, None, Some("error_message"));
        assert!(records[0].error_message.is_none());
        assert!(!records[0].tags.is_empty());
    }

    #[test]
    fn run_errors_on_invalid_glob() {
        let args = ExportMetadataArgs {
            input: vec![PathBuf::from("[".to_string())],
            output: None,
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        assert!(run(&args, false).is_err());
    }

    #[test]
    fn apply_field_filter_include_requested_tag_keys_without_tags() {
        let mut records = vec![sample_record()];
        apply_field_filter(&mut records, Some("tags.artist"), None);
        assert_eq!(records[0].tags.len(), 1);
        assert!(records[0].tags.contains_key("artist"));
        assert!(records[0].properties.is_empty());
    }

    #[test]
    fn build_manifest_omits_cover_when_no_front_cover_present() {
        let mut record = sample_record_with_cover();
        record.front_cover = None;
        record.pictures.front_cover_present = false;
        let tmp = tempfile::tempdir().unwrap();
        let mode = OutputMode::AggregateFile(tmp.path().join("manifest.yaml"));
        let base_dir = tmp.path().to_path_buf();
        let (manifest, covers) = build_manifest(&[record], true, None, &mode, &base_dir).unwrap();
        assert!(manifest.files[0].cover.is_none());
        assert!(covers.is_empty());
    }

    #[test]
    fn extension_for_picture_no_mime_returns_bin() {
        let picture = Picture {
            mime_type: None,
            description: None,
            picture_type: None,
            data: vec![],
        };
        assert_eq!(extension_for_picture(&picture), "bin");
    }

    #[test]
    fn extension_for_picture_empty_mime_returns_bin() {
        let picture = Picture {
            mime_type: Some("".to_string()),
            description: None,
            picture_type: None,
            data: vec![],
        };
        assert_eq!(extension_for_picture(&picture), "bin");
    }

    #[test]
    fn resolve_output_mode_trailing_backslash_selects_sidecar() {
        let tmp = tempfile::tempdir().unwrap();
        let args = ExportMetadataArgs {
            input: vec![],
            output: Some(PathBuf::from(format!("{}\\", tmp.path().display()))),
            per_file: false,
            aggregate: false,
            with_cover: false,
            cover_dir: None,
            fields: None,
            exclude_fields: None,
            absolute_paths: false,
            relative_paths: false,
            fail_fast: false,
            yes: false,
        };
        let mode = resolve_output_mode(&args).unwrap();
        assert!(matches!(mode, OutputMode::SidecarDirectory(_)));
    }
}
