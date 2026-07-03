use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::taglib::{AudioProperties, Metadata, Picture};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
}

#[derive(Serialize)]
struct InfoOutput<'a> {
    file: &'a str,
    audio: Option<&'a AudioProperties>,
    tags: Map<String, Value>,
    pictures: Vec<PictureSummary>,
}

#[derive(Serialize, Clone)]
pub struct PictureSummary {
    mime_type: Option<String>,
    picture_type: Option<String>,
    size_bytes: usize,
}

impl PictureSummary {
    pub fn from_picture(pic: &Picture) -> Self {
        Self {
            mime_type: pic.mime_type.clone(),
            picture_type: pic.picture_type.clone(),
            size_bytes: pic.data.len(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ExportRecord {
    pub file_path: String,
    pub file_name: String,
    pub relative_path: String,
    pub file_format: String,
    pub tags: BTreeMap<String, String>,
    pub properties: BTreeMap<String, Vec<String>>,
    pub audio: Option<AudioProperties>,
    pub pictures: PicturesSummary,
    #[serde(skip)]
    pub front_cover: Option<Picture>,
    pub read_status: String,
    pub error_message: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PicturesSummary {
    pub count: usize,
    pub front_cover_present: bool,
    pub summaries: Vec<PictureSummary>,
}

#[derive(Serialize, Clone)]
pub struct ExportOutput {
    pub export_timestamp: String,
    pub generator: String,
    pub summary: ExportSummary,
    pub records: Vec<ExportRecord>,
    pub failures: Vec<FailureRecord>,
}

#[derive(Serialize, Clone)]
pub struct ExportSummary {
    pub total: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Serialize, Clone)]
pub struct FailureRecord {
    pub file_path: String,
    pub read_status: String,
    pub error_category: String,
    pub error_message: String,
}

impl ExportRecord {
    pub fn from_metadata(
        metadata: &Metadata,
        file_path: &str,
        file_name: &str,
        relative_path: &str,
        file_format: &str,
    ) -> Self {
        let front_cover_present = metadata
            .pictures
            .iter()
            .any(|p| p.picture_type.as_deref() == Some("Front Cover"));
        let front_cover = metadata
            .pictures
            .iter()
            .find(|p| p.picture_type.as_deref() == Some("Front Cover"))
            .cloned();
        Self {
            file_path: file_path.to_string(),
            file_name: file_name.to_string(),
            relative_path: relative_path.to_string(),
            file_format: file_format.to_string(),
            tags: normalize_tags(&metadata.properties),
            properties: metadata.properties.clone(),
            audio: metadata.audio.clone(),
            pictures: PicturesSummary {
                count: metadata.pictures.len(),
                front_cover_present,
                summaries: metadata
                    .pictures
                    .iter()
                    .map(PictureSummary::from_picture)
                    .collect(),
            },
            front_cover,
            read_status: "ok".to_string(),
            error_message: None,
        }
    }
}

/// Map common uppercase TagLib property keys to normalized lowercase/snake_case
/// keys. Unknown keys are omitted from the normalized map; the raw `properties`
/// map always retains them.
pub fn normalize_tags(properties: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, String> {
    let mut tags = BTreeMap::new();
    for (key, values) in properties {
        if values.is_empty() {
            continue;
        }
        let normalized_key = match key.as_str() {
            "TITLE" => "title",
            "ARTIST" => "artist",
            "ALBUM" => "album",
            "ALBUMARTIST" => "album_artist",
            "GENRE" => "genre",
            "DATE" => "date",
            "YEAR" => "year",
            "TRACKNUMBER" => "track_number",
            "TRACKTOTAL" => "track_total",
            "DISCNUMBER" => "disc_number",
            "DISCTOTAL" => "disc_total",
            "COMPOSER" => "composer",
            "PUBLISHER" => "publisher",
            "COPYRIGHT" => "copyright",
            "COMMENT" => "comment",
            "DESCRIPTION" => "description",
            "URL" => "url",
            "ISRC" => "isrc",
            "LABEL" => "label",
            "CATALOGNUMBER" => "catalog_number",
            "LYRICS" => "lyrics",
            "LANGUAGE" => "language",
            "EXPLICIT" => "explicit",
            "BPM" => "bpm",
            "INITIALKEY" | "KEY" => "initial_key",
            _ => continue,
        };
        tags.insert(normalized_key.to_string(), values.join("; "));
    }
    tags
}

pub fn format_info(metadata: &Metadata, file: &str, format: OutputFormat) -> String {
    match format {
        OutputFormat::Table => format_info_table(metadata, file),
        OutputFormat::Json => format_info_json(metadata, file),
        OutputFormat::Yaml => format_info_yaml(metadata, file),
    }
}

pub fn format_get(metadata: &Metadata, keys: &[String], format: OutputFormat) -> String {
    let selected: BTreeMap<String, Vec<String>> = metadata
        .properties
        .iter()
        .filter(|(k, _)| keys.is_empty() || keys.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    match format {
        OutputFormat::Table => selected
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v.join("; ")))
            .collect::<Vec<_>>()
            .join("\n"),
        OutputFormat::Json => serde_json::to_string_pretty(&selected).unwrap_or_default(),
        OutputFormat::Yaml => serde_yaml::to_string(&selected).unwrap_or_default(),
    }
}

fn format_info_table(metadata: &Metadata, file: &str) -> String {
    let mut lines = vec![format!("File: {}", file)];
    if let Some(audio) = &metadata.audio {
        lines.push("\nAudio:".to_string());
        lines.push(format!("  Duration:    {}s", audio.length_seconds));
        lines.push(format!("  Bitrate:     {} kbps", audio.bitrate_kbps));
        lines.push(format!("  Sample rate: {} Hz", audio.sample_rate_hz));
        lines.push(format!("  Channels:    {}", audio.channels));
    }
    lines.push("\nTags:".to_string());
    for (key, values) in &metadata.properties {
        let value = values.join("; ");
        lines.push(format!("  {:20} {}", key, value));
    }
    lines.push("\nPictures:".to_string());
    for pic in &metadata.pictures {
        lines.push(format!(
            "  {}  {}  {} KB",
            pic.picture_type.as_deref().unwrap_or("Unknown"),
            pic.mime_type.as_deref().unwrap_or("unknown"),
            pic.data.len() / 1024
        ));
    }
    lines.join("\n")
}

fn format_info_json(metadata: &Metadata, file: &str) -> String {
    let output = build_info_output(metadata, file);
    serde_json::to_string_pretty(&output).unwrap_or_default()
}

fn format_info_yaml(metadata: &Metadata, file: &str) -> String {
    let output = build_info_output(metadata, file);
    serde_yaml::to_string(&output).unwrap_or_default()
}

fn build_info_output<'a>(metadata: &'a Metadata, file: &'a str) -> InfoOutput<'a> {
    let tags = serde_json::to_value(&metadata.properties)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let pictures = metadata
        .pictures
        .iter()
        .map(PictureSummary::from_picture)
        .collect();

    InfoOutput {
        file,
        audio: metadata.audio.as_ref(),
        tags,
        pictures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taglib::Tags;

    fn sample_metadata() -> Metadata {
        Metadata {
            tags: crate::taglib::Tags {
                title: Some("Title".to_string()),
                artist: Some("Artist".to_string()),
                album: Some("Album".to_string()),
            },
            properties: {
                let mut m = BTreeMap::new();
                m.insert("TITLE".to_string(), vec!["Title".to_string()]);
                m.insert("ARTIST".to_string(), vec!["Artist".to_string()]);
                m
            },
            pictures: vec![Picture {
                mime_type: Some("image/jpeg".to_string()),
                description: Some("cover".to_string()),
                picture_type: Some("Front Cover".to_string()),
                data: vec![0u8; 2048],
            }],
            audio: Some(AudioProperties {
                length_seconds: 120,
                bitrate_kbps: 320,
                sample_rate_hz: 44100,
                channels: 2,
            }),
        }
    }

    #[test]
    fn format_info_json_includes_picture_summary() {
        let md = sample_metadata();
        let out = format_info_json(&md, "/tmp/test.mp3");
        assert!(out.contains("\"file\": \"/tmp/test.mp3\""));
        assert!(out.contains("\"mime_type\": \"image/jpeg\""));
        assert!(out.contains("\"size_bytes\": 2048"));
        assert!(!out.contains("\"data\""));
    }

    #[test]
    fn format_info_yaml_includes_picture_summary() {
        let md = sample_metadata();
        let out = format_info_yaml(&md, "/tmp/test.mp3");
        assert!(out.contains("/tmp/test.mp3"));
        assert!(out.contains("mime_type"));
        assert!(out.contains("size_bytes"));
    }

    #[test]
    fn format_info_table_includes_all_sections() {
        let md = sample_metadata();
        let out = format_info(&md, "/tmp/test.mp3", OutputFormat::Table);
        assert!(out.contains("File: /tmp/test.mp3"));
        assert!(out.contains("Audio:"));
        assert!(out.contains("Tags:"));
        assert!(out.contains("Pictures:"));
        assert!(out.contains("Front Cover"));
        assert!(out.contains("image/jpeg"));
    }

    #[test]
    fn format_info_table_without_audio_or_pictures() {
        let md = Metadata {
            tags: Tags::default(),
            properties: {
                let mut m = BTreeMap::new();
                m.insert("TITLE".to_string(), vec!["Title".to_string()]);
                m
            },
            pictures: vec![],
            audio: None,
        };
        let out = format_info(&md, "/tmp/test.mp3", OutputFormat::Table);
        assert!(out.contains("File: /tmp/test.mp3"));
        assert!(!out.contains("Audio:"));
        assert!(out.contains("Pictures:"));
    }

    #[test]
    fn format_get_filters_keys() {
        let md = sample_metadata();
        let out = format_get(&md, &["TITLE".to_string()], OutputFormat::Table);
        assert!(out.contains("TITLE: Title"));
        assert!(!out.contains("ARTIST"));
    }

    #[test]
    fn format_get_empty_keys_returns_all() {
        let md = sample_metadata();
        let out = format_get(&md, &[], OutputFormat::Table);
        assert!(out.contains("TITLE"));
        assert!(out.contains("ARTIST"));
    }

    #[test]
    fn format_get_json_and_yaml() {
        let md = sample_metadata();
        let json = format_get(&md, &["TITLE".to_string()], OutputFormat::Json);
        assert!(json.contains("\"TITLE\""));

        let yaml = format_get(&md, &["TITLE".to_string()], OutputFormat::Yaml);
        assert!(yaml.contains("TITLE:"));
    }

    #[test]
    fn format_info_table_displays_duration() {
        let md = Metadata {
            tags: Tags::default(),
            properties: BTreeMap::new(),
            pictures: vec![],
            audio: Some(AudioProperties {
                length_seconds: 1,
                bitrate_kbps: 128,
                sample_rate_hz: 44100,
                channels: 2,
            }),
        };
        let out = format_info(&md, "/tmp/one_second.mp3", OutputFormat::Table);
        assert!(out.contains("Duration:    1s"));
    }

    #[test]
    fn format_info_table_displays_anomalous_duration() {
        // Covers the case where TagLib reports a value (e.g. 5s) that does
        // not match the real audio duration. The formatter should render the
        // reported value faithfully.
        let md = Metadata {
            tags: Tags::default(),
            properties: BTreeMap::new(),
            pictures: vec![],
            audio: Some(AudioProperties {
                length_seconds: 5,
                bitrate_kbps: 128,
                sample_rate_hz: 44100,
                channels: 2,
            }),
        };
        let out = format_info(&md, "/tmp/five_second.mp3", OutputFormat::Table);
        assert!(out.contains("Duration:    5s"));
    }

    #[test]
    fn normalize_tags_maps_common_keys() {
        let mut props = BTreeMap::new();
        props.insert("TITLE".to_string(), vec!["Song".to_string()]);
        props.insert("ARTIST".to_string(), vec!["Artist".to_string()]);
        props.insert("ALBUMARTIST".to_string(), vec!["Album Artist".to_string()]);
        props.insert("UNKNOWN".to_string(), vec!["x".to_string()]);

        let tags = normalize_tags(&props);
        assert_eq!(tags.get("title"), Some(&"Song".to_string()));
        assert_eq!(tags.get("artist"), Some(&"Artist".to_string()));
        assert_eq!(tags.get("album_artist"), Some(&"Album Artist".to_string()));
        assert!(!tags.contains_key("unknown"));
    }

    #[test]
    fn normalize_tags_joins_multiple_values() {
        let mut props = BTreeMap::new();
        props.insert(
            "ARTIST".to_string(),
            vec!["Artist A".to_string(), "Artist B".to_string()],
        );

        let tags = normalize_tags(&props);
        assert_eq!(tags.get("artist"), Some(&"Artist A; Artist B".to_string()));
    }

    #[test]
    fn normalize_tags_skips_empty_values() {
        let mut props = BTreeMap::new();
        props.insert("TITLE".to_string(), vec![]);
        props.insert("ARTIST".to_string(), vec!["Artist".to_string()]);

        let tags = normalize_tags(&props);
        assert!(!tags.contains_key("title"));
        assert_eq!(tags.get("artist"), Some(&"Artist".to_string()));
    }

    #[test]
    fn normalize_tags_maps_key_aliases() {
        let mut props = BTreeMap::new();
        props.insert("INITIALKEY".to_string(), vec!["C#".to_string()]);

        let tags = normalize_tags(&props);
        assert_eq!(tags.get("initial_key"), Some(&"C#".to_string()));
    }

    #[test]
    fn export_record_from_metadata_populates_summary() {
        let md = sample_metadata();
        let record =
            ExportRecord::from_metadata(&md, "/tmp/test.mp3", "test.mp3", "test.mp3", "mp3");
        assert_eq!(record.file_path, "/tmp/test.mp3");
        assert_eq!(record.file_name, "test.mp3");
        assert_eq!(record.relative_path, "test.mp3");
        assert_eq!(record.file_format, "mp3");
        assert_eq!(record.tags.get("title"), Some(&"Title".to_string()));
        assert_eq!(
            record.properties.get("TITLE"),
            Some(&vec!["Title".to_string()])
        );
        assert!(record.pictures.front_cover_present);
        assert_eq!(record.pictures.count, 1);
        assert_eq!(record.pictures.summaries.len(), 1);
        assert_eq!(record.read_status, "ok");
        assert!(record.error_message.is_none());
    }
}
