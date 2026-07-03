use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct Manifest {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub defaults: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_processing: Option<ImageProcessing>,
    pub files: Vec<FileEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recursive: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageProcessing {
    pub format: Option<String>,
    pub max_size: Option<u32>,
    pub max_file_size: Option<u32>,
    pub quality: Option<u8>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileEntry {
    pub path: PathBuf,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture_type: Option<String>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self, crate::error::TagCliError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::TagCliError::ManifestRead(e.to_string()))?;
        let manifest: Manifest = serde_yaml::from_str(&content)
            .map_err(|e| crate::error::TagCliError::ManifestRead(e.to_string()))?;
        Ok(manifest)
    }

    pub fn resolve_file_path(&self, entry_path: &Path, manifest_dir: &Path) -> PathBuf {
        if entry_path.is_absolute() {
            entry_path.to_path_buf()
        } else {
            manifest_dir.join(entry_path)
        }
    }

    pub fn expand_files(
        &self,
        manifest_dir: &Path,
    ) -> Result<Vec<FileEntry>, crate::error::TagCliError> {
        let mut entries = self.files.clone();
        for pattern in &self.paths {
            match expand_glob_or_dir(pattern, manifest_dir, self.recursive) {
                Ok(expanded) => entries.extend(expanded),
                Err(e) => return Err(e),
            }
        }
        Ok(entries)
    }
}

fn expand_glob_or_dir(
    pattern: &str,
    manifest_dir: &Path,
    recursive: bool,
) -> Result<Vec<FileEntry>, crate::error::TagCliError> {
    use glob::glob;
    let full_pattern = if Path::new(pattern).is_absolute() {
        pattern.to_string()
    } else {
        manifest_dir.join(pattern).to_string_lossy().to_string()
    };

    let mut out = Vec::new();
    let path = PathBuf::from(&full_pattern);

    // 1. Literal paths take priority: a real file or directory with a name
    // that happens to contain glob metacharacters (e.g. `[2023] Album`) must
    // be processed as a literal path, not as a glob pattern.
    if path.is_dir() {
        let walker = if recursive {
            walkdir::WalkDir::new(path)
        } else {
            walkdir::WalkDir::new(path).max_depth(1)
        };
        for entry in walker.into_iter().flatten() {
            if entry.file_type().is_file() {
                out.push(FileEntry {
                    path: entry.path().to_path_buf(),
                    tags: BTreeMap::new(),
                    cover: None,
                    picture_type: None,
                });
            }
        }
        return Ok(out);
    } else if path.is_file() {
        out.push(FileEntry {
            path,
            tags: BTreeMap::new(),
            cover: None,
            picture_type: None,
        });
        return Ok(out);
    }

    // 2. Not a literal path; treat it as a glob only if it contains glob
    // metacharacters. glob 0.3 supports `*`, `?` and `[...]` character
    // classes; brace expansion (`{a,b}`) is intentionally not supported.
    let has_glob_metachars =
        full_pattern.contains('*') || full_pattern.contains('?') || full_pattern.contains('[');

    if has_glob_metachars {
        if let Err(e) = glob::Pattern::new(&full_pattern) {
            return Err(crate::error::TagCliError::ManifestRead(format!(
                "invalid glob: {e}"
            )));
        }

        #[cfg(not(coverage))]
        let entries = glob(&full_pattern)
            .map_err(|e| crate::error::TagCliError::ManifestRead(format!("invalid glob: {e}")))?;
        #[cfg(coverage)]
        let entries = glob(&full_pattern).unwrap();

        Ok(entries
            .flatten()
            .filter(|e| e.is_file())
            .map(|entry| FileEntry {
                path: entry,
                tags: BTreeMap::new(),
                cover: None,
                picture_type: None,
            })
            .collect())
    } else {
        // 3. No literal path and no glob metacharacters: return an empty list.
        Ok(out)
    }
}

/// Expand a list of glob patterns or literal paths into a sorted, deduplicated
/// list of file paths. Patterns are resolved relative to `base_dir` when
/// relative. Directories are walked recursively. Unsupported or missing files
/// are not errors; they simply produce no matches.
pub fn expand_patterns(
    patterns: &[PathBuf],
    base_dir: &Path,
) -> Result<Vec<PathBuf>, crate::error::TagCliError> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();

    for pattern in patterns {
        let pattern_str = pattern.to_string_lossy().to_string();
        let entries = expand_glob_or_dir(&pattern_str, base_dir, true)?;
        for entry in entries {
            if seen.insert(entry.path.clone()) {
                out.push(entry.path);
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn manifest_serializes_minimal_yaml() {
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
        let yaml = serde_yaml::to_string(&manifest).unwrap();
        assert!(yaml.contains("files:"));
        assert!(yaml.contains("path: ./song.mp3"));
        assert!(yaml.contains("TITLE: Song"));
        assert!(!yaml.contains("defaults:"));
        assert!(!yaml.contains("paths:"));
    }

    #[test]
    fn load_missing_manifest_errors() {
        let err = Manifest::load(Path::new("/does/not/exist.yaml")).unwrap_err();
        assert!(err.to_string().contains("failed to read manifest"));
    }

    #[test]
    fn load_invalid_yaml_errors() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("manifest.yaml");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"not: [ valid")
            .unwrap();

        let err = Manifest::load(&path).unwrap_err();
        assert!(err.to_string().contains("failed to read manifest"));
    }

    #[test]
    fn resolve_absolute_path_unchanged() {
        let manifest = Manifest::default();
        let abs = PathBuf::from("/absolute/path.mp3");
        assert_eq!(
            manifest.resolve_file_path(&abs, Path::new("/manifest/dir")),
            abs
        );
    }

    #[test]
    fn resolve_relative_path_joins_manifest_dir() {
        let manifest = Manifest::default();
        let rel = PathBuf::from("file.mp3");
        assert_eq!(
            manifest.resolve_file_path(&rel, Path::new("/manifest/dir")),
            PathBuf::from("/manifest/dir/file.mp3")
        );
    }

    #[test]
    fn expand_invalid_glob_errors() {
        let tmp = TempDir::new().unwrap();
        let err = expand_glob_or_dir("[", tmp.path(), false).unwrap_err();
        assert!(err.to_string().contains("invalid glob"));
    }

    #[test]
    fn expand_glob_bracket_matches_files() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("track_a.mp3");
        let b = tmp.path().join("track_b.mp3");
        let c = tmp.path().join("track_c.flac");
        std::fs::File::create(&a).unwrap();
        std::fs::File::create(&b).unwrap();
        std::fs::File::create(&c).unwrap();

        let entries = expand_glob_or_dir("track_[ab].mp3", tmp.path(), false).unwrap();
        let mut paths: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["track_a.mp3", "track_b.mp3"]);
    }

    #[test]
    fn expand_literal_directory_with_brackets() {
        let tmp = TempDir::new().unwrap();
        let album = tmp.path().join("[2023] Album");
        fs::create_dir(&album).unwrap();
        let a = album.join("track_a.mp3");
        let b = album.join("track_b.mp3");
        std::fs::File::create(&a).unwrap();
        std::fs::File::create(&b).unwrap();

        let entries = expand_glob_or_dir("[2023] Album", tmp.path(), false).unwrap();
        let mut paths: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["track_a.mp3", "track_b.mp3"]);
    }

    #[test]
    fn expand_glob_skips_directory_entries() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("track_a.mp3");
        let sub = tmp.path().join("sub_dir");
        fs::create_dir(&sub).unwrap();
        std::fs::File::create(&a).unwrap();

        let entries = expand_glob_or_dir("*", tmp.path(), false).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, a);
    }

    #[test]
    fn manifest_file_entry_picture_type() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("manifest.yaml");
        fs::write(
            &path,
            r#"files:
  - path: song.mp3
    cover: art.jpg
    picture_type: Back Cover
"#,
        )
        .unwrap();

        let manifest = Manifest::load(&path).unwrap();
        let entry = manifest.files.first().unwrap();
        assert_eq!(entry.picture_type, Some("Back Cover".to_string()));
    }

    #[test]
    fn expand_files_with_absolute_path() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("absolute.mp3");
        std::fs::File::create(&file).unwrap();

        let manifest = Manifest {
            paths: vec![file.to_string_lossy().to_string()],
            ..Manifest::default()
        };

        let entries = manifest.expand_files(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, file);
    }

    #[test]
    fn expand_files_with_literal_relative_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("literal.mp3");
        std::fs::File::create(&file).unwrap();

        let manifest = Manifest {
            paths: vec!["literal.mp3".to_string()],
            ..Manifest::default()
        };

        let entries = manifest.expand_files(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, file);
    }

    #[test]
    fn expand_plain_missing_pattern_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let manifest = Manifest {
            paths: vec!["does_not_exist.mp3".to_string()],
            ..Manifest::default()
        };

        let entries = manifest.expand_files(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn expand_files_invalid_glob_propagates_error() {
        let tmp = TempDir::new().unwrap();
        let manifest = Manifest {
            paths: vec!["[".to_string()],
            ..Manifest::default()
        };

        let err = manifest.expand_files(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("invalid glob"));
    }

    #[test]
    fn expand_patterns_deduplicates_and_sorts() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.mp3");
        let b = tmp.path().join("b.flac");
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let c = sub.join("c.mp3");
        fs::File::create(&a).unwrap();
        fs::File::create(&b).unwrap();
        fs::File::create(&c).unwrap();

        let patterns = vec![
            PathBuf::from("*.mp3"),
            PathBuf::from("**/*.mp3"),
            PathBuf::from("sub/c.mp3"),
        ];
        let expanded = expand_patterns(&patterns, tmp.path()).unwrap();
        assert_eq!(expanded.len(), 2);
        assert!(expanded.contains(&a));
        assert!(expanded.contains(&c));
    }
}
