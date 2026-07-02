use std::collections::BTreeMap;
use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use taglib_rs::test_utils::{generate_flac, generate_mp3};
use taglib_rs::{
    AudioProperties, CoverWriteAction, Metadata, Picture, TagError, Tags, read_metadata_from_path,
    take_taglib_borrowed_string, take_taglib_string, write_full_properties_to_path,
    write_properties_to_path,
};
use tempfile::TempDir;

fn bad_open_path(tmp: &TempDir) -> PathBuf {
    // An excessively long file name should cause taglib_file_new to
    // return null on all platforms.
    let name = "a".repeat(4096);
    tmp.path().join(name)
}

#[test]
fn read_metadata_from_empty_file_is_invalid() {
    let tmp = std::env::temp_dir().join(format!(
        "tag-cli-empty-{}-{}.bin",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::write(&tmp, []).unwrap();
    let err = read_metadata_from_path(&tmp).unwrap_err();
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
#[cfg(unix)]
fn read_metadata_null_byte_path_is_invalid() {
    let bad = Path::new(OsStr::from_bytes(b"\x00invalid"));
    let err = read_metadata_from_path(bad).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn read_metadata_directory_fails() {
    let tmp = TempDir::new().unwrap();
    let err = read_metadata_from_path(tmp.path()).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
#[cfg(unix)]
fn write_properties_null_byte_path_is_invalid() {
    let bad = Path::new(OsStr::from_bytes(b"\x00invalid"));
    let err = write_properties_to_path(bad, &BTreeMap::new(), CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
#[cfg(unix)]
fn write_full_properties_null_byte_path_is_invalid() {
    let bad = Path::new(OsStr::from_bytes(b"\x00invalid"));
    let err =
        write_full_properties_to_path(bad, &BTreeMap::new(), CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn write_properties_null_byte_key_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("TITLE\x00BAD".to_string(), vec!["T".to_string()]);
    let err = write_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn write_properties_null_byte_value_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T\x00BAD".to_string()]);
    let err = write_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn write_properties_null_byte_second_value_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert(
        "ARTIST".to_string(),
        vec!["A".to_string(), "B\x00BAD".to_string()],
    );
    let err = write_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn write_full_properties_null_byte_key_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("TITLE\x00BAD".to_string(), vec!["T".to_string()]);
    let err = write_full_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn write_full_properties_null_byte_value_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T\x00BAD".to_string()]);
    let err = write_full_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn write_full_properties_null_byte_second_value_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert(
        "ARTIST".to_string(),
        vec!["A".to_string(), "B\x00BAD".to_string()],
    );
    let err = write_full_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn set_cover_with_null_byte_strings_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let cover = Picture {
        mime_type: Some("image/jpeg\x00bad".to_string()),
        description: Some("Cover".to_string()),
        picture_type: Some("Front Cover".to_string()),
        data: vec![0u8; 100],
    };

    let err = write_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover))
        .unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn cover_description_with_null_byte_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let cover = Picture {
        mime_type: Some("image/jpeg".to_string()),
        description: Some("Cover\x00bad".to_string()),
        picture_type: Some("Front Cover".to_string()),
        data: vec![0u8; 100],
    };

    let err = write_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover))
        .unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn cover_picture_type_with_null_byte_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let cover = Picture {
        mime_type: Some("image/jpeg".to_string()),
        description: Some("Cover".to_string()),
        picture_type: Some("Front Cover\x00bad".to_string()),
        data: vec![0u8; 100],
    };

    let err = write_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover))
        .unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn full_replace_cover_mime_with_null_byte_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let cover = Picture {
        mime_type: Some("image/jpeg\x00bad".to_string()),
        description: Some("Cover".to_string()),
        picture_type: Some("Front Cover".to_string()),
        data: vec![0u8; 100],
    };

    let err = write_full_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover))
        .unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn full_replace_cover_description_with_null_byte_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let cover = Picture {
        mime_type: Some("image/jpeg".to_string()),
        description: Some("Cover\x00bad".to_string()),
        picture_type: Some("Front Cover".to_string()),
        data: vec![0u8; 100],
    };

    let err = write_full_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover))
        .unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn full_replace_cover_picture_type_with_null_byte_fails() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let cover = Picture {
        mime_type: Some("image/jpeg".to_string()),
        description: Some("Cover".to_string()),
        picture_type: Some("Front Cover\x00bad".to_string()),
        data: vec![0u8; 100],
    };

    let err = write_full_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover))
        .unwrap_err();
    assert_eq!(err, TagError::InvalidPath);
}

#[test]
fn take_taglib_string_null_returns_none() {
    unsafe {
        assert!(take_taglib_string(std::ptr::null_mut()).is_none());
    }
}

#[test]
fn take_taglib_borrowed_string_null_returns_none() {
    unsafe {
        assert!(take_taglib_borrowed_string(std::ptr::null()).is_none());
    }
}

#[test]
fn take_taglib_borrowed_string_empty_returns_none() {
    unsafe {
        let empty: &[u8] = b"\0";
        assert!(
            take_taglib_borrowed_string(empty.as_ptr().cast::<std::os::raw::c_char>()).is_none()
        );
    }
}

#[test]
fn directory_path_fails_to_open() {
    let tmp = TempDir::new().unwrap();
    let err = read_metadata_from_path(tmp.path()).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn write_properties_to_directory_fails_to_open() {
    let tmp = TempDir::new().unwrap();
    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err = write_properties_to_path(tmp.path(), &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn write_full_properties_to_directory_fails_to_open() {
    let tmp = TempDir::new().unwrap();
    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err =
        write_full_properties_to_path(tmp.path(), &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn read_metadata_missing_file_fails_to_open() {
    let tmp = TempDir::new().unwrap();
    let bad = bad_open_path(&tmp);
    let err = read_metadata_from_path(&bad).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn write_properties_missing_file_fails_to_open() {
    let tmp = TempDir::new().unwrap();
    let bad = bad_open_path(&tmp);
    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err = write_properties_to_path(&bad, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn write_full_properties_missing_file_fails_to_open() {
    let tmp = TempDir::new().unwrap();
    let bad = bad_open_path(&tmp);
    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err = write_full_properties_to_path(&bad, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn write_properties_to_empty_file_is_invalid() {
    let tmp = TempDir::new().unwrap();
    let empty = tmp.path().join("empty.bin");
    std::fs::write(&empty, b"not audio").unwrap();
    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err = write_properties_to_path(&empty, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn write_full_properties_to_empty_file_is_invalid() {
    let tmp = TempDir::new().unwrap();
    let empty = tmp.path().join("empty.bin");
    std::fs::write(&empty, b"not audio").unwrap();
    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err = write_full_properties_to_path(&empty, &props, CoverWriteAction::Keep).unwrap_err();
    assert_eq!(err, TagError::InvalidFile);
}

#[test]
fn write_and_read_multiple_values() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("ARTIST".to_string(), vec!["A".to_string(), "B".to_string()]);
    write_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap();

    let metadata = read_metadata_from_path(&input).unwrap();
    let artists = metadata.properties.get("ARTIST").expect("ARTIST missing");
    assert!(artists.contains(&"A".to_string()));
    assert!(artists.contains(&"B".to_string()));
}

#[test]
fn full_replace_skips_empty_values() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["   ".to_string()]);
    write_full_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap();

    let metadata = read_metadata_from_path(&input).unwrap();
    assert!(!metadata.properties.contains_key("TITLE"));
}

#[test]
fn write_properties_skips_empty_trimmed_values() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["   ".to_string()]);
    write_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap();

    let metadata = read_metadata_from_path(&input).unwrap();
    assert!(!metadata.properties.contains_key("TITLE"));
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn incremental_save_fails_on_read_only_file() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);
    let mut perms = std::fs::metadata(&input).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&input, perms).unwrap();

    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err = write_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();

    let mut perms = std::fs::metadata(&input).unwrap().permissions();
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(&input, perms);
    assert_eq!(err, TagError::SaveFailed);
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn full_replace_save_fails_on_read_only_file() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);
    let mut perms = std::fs::metadata(&input).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&input, perms).unwrap();

    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["T".to_string()]);
    let err = write_full_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap_err();

    let mut perms = std::fs::metadata(&input).unwrap().permissions();
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(&input, perms);
    assert_eq!(err, TagError::SaveFailed);
}

#[test]
fn full_replace_appends_multiple_values() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let mut props = BTreeMap::new();
    props.insert("ARTIST".to_string(), vec!["A".to_string(), "B".to_string()]);
    write_full_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap();

    let metadata = read_metadata_from_path(&input).unwrap();
    let artists = metadata.properties.get("ARTIST").expect("ARTIST missing");
    assert!(artists.contains(&"A".to_string()));
    assert!(artists.contains(&"B".to_string()));
}

#[test]
fn read_flac_properties_frees_value_lists() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.flac");
    generate_flac(&input);

    let mut props = BTreeMap::new();
    props.insert("TITLE".to_string(), vec!["Flac Title".to_string()]);
    write_properties_to_path(&input, &props, CoverWriteAction::Keep).unwrap();

    let metadata = read_metadata_from_path(&input).unwrap();
    assert_eq!(
        metadata.properties.get("TITLE"),
        Some(&vec!["Flac Title".to_string()])
    );
}

#[test]
fn empty_cover_data_is_skipped_when_reading() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    let cover = Picture {
        mime_type: Some("image/jpeg".to_string()),
        description: Some("Cover".to_string()),
        picture_type: Some("Front Cover".to_string()),
        data: vec![],
    };
    write_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover)).unwrap();

    let metadata = read_metadata_from_path(&input).unwrap();
    assert!(metadata.pictures.is_empty());
}

#[test]
fn debug_derives_are_exercised() {
    let _ = format!(
        "{:?}",
        Metadata {
            tags: Tags::default(),
            properties: std::collections::BTreeMap::new(),
            pictures: vec![Picture {
                mime_type: None,
                description: None,
                picture_type: None,
                data: vec![],
            }],
            audio: Some(AudioProperties {
                length_seconds: 0,
                bitrate_kbps: 0,
                sample_rate_hz: 0,
                channels: 0,
            }),
        }
    );
    for err in [
        TagError::InvalidPath,
        TagError::OpenFailed,
        TagError::MissingTag,
        TagError::InvalidFile,
        TagError::SaveFailed,
        TagError::CoverSetFailed,
    ] {
        let _ = format!("{:?}", err);
    }
    for action in [
        CoverWriteAction::Keep,
        CoverWriteAction::Clear,
        CoverWriteAction::Set(Picture {
            mime_type: None,
            description: None,
            picture_type: None,
            data: vec![],
        }),
    ] {
        let _ = format!("{:?}", action);
    }
    for err in [
        TagError::InvalidPath,
        TagError::OpenFailed,
        TagError::MissingTag,
        TagError::InvalidFile,
        TagError::SaveFailed,
        TagError::CoverSetFailed,
    ] {
        let _: Option<&(dyn std::error::Error + 'static)> = std::error::Error::source(&err);
        let _ = err.to_string();
    }
}

#[test]
fn clone_derives_are_exercised() {
    let _ = Tags::default().clone();
    let _ = AudioProperties {
        length_seconds: 0,
        bitrate_kbps: 0,
        sample_rate_hz: 0,
        channels: 0,
    }
    .clone();
    let picture = Picture {
        mime_type: None,
        description: None,
        picture_type: None,
        data: vec![],
    };
    let _ = picture.clone();
    let metadata = Metadata {
        tags: Tags::default(),
        properties: BTreeMap::new(),
        pictures: vec![picture.clone()],
        audio: None,
    };
    let _ = metadata.clone();
    for err in [
        TagError::InvalidPath,
        TagError::OpenFailed,
        TagError::MissingTag,
        TagError::InvalidFile,
        TagError::SaveFailed,
        TagError::CoverSetFailed,
    ] {
        let _ = err.clone();
    }
}

#[test]
fn partial_eq_derives_are_exercised() {
    let tags = Tags::default();
    assert_eq!(tags, tags.clone());

    let audio = AudioProperties {
        length_seconds: 0,
        bitrate_kbps: 0,
        sample_rate_hz: 0,
        channels: 0,
    };
    assert_eq!(audio, audio.clone());

    let picture = Picture {
        mime_type: None,
        description: None,
        picture_type: None,
        data: vec![],
    };
    assert_eq!(picture, picture.clone());

    let metadata = Metadata {
        tags: Tags::default(),
        properties: BTreeMap::new(),
        pictures: vec![picture.clone()],
        audio: None,
    };
    assert_eq!(metadata, metadata.clone());

    let mut other = metadata.clone();
    other.tags.title = Some("x".to_string());
    assert_ne!(metadata, other);

    assert_eq!(TagError::InvalidPath, TagError::InvalidPath);
    assert_ne!(TagError::InvalidPath, TagError::InvalidFile);
}

#[test]
fn write_full_properties_clears_cover() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp3");
    generate_mp3(&input);

    // Set a cover first so that clearing it is meaningful.
    let cover = Picture {
        mime_type: Some("image/jpeg".to_string()),
        description: Some("Cover".to_string()),
        picture_type: Some("Front Cover".to_string()),
        data: vec![0u8; 100],
    };
    write_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover)).unwrap();

    write_full_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Clear).unwrap();

    let metadata = read_metadata_from_path(&input).unwrap();
    assert!(metadata.pictures.is_empty());
}
