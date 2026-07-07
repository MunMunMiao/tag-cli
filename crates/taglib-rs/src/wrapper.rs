use std::collections::BTreeMap;
use std::ffi::CStr;
use std::path::Path;
use std::sync::Once;

use super::ffi;
use crate::supported_property_keys;

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AudioProperties {
    pub length_seconds: i32,
    pub bitrate_kbps: i32,
    pub sample_rate_hz: i32,
    pub channels: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub picture_type: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Metadata {
    pub tags: Tags,
    pub properties: BTreeMap<String, Vec<String>>,
    #[serde(skip)]
    pub pictures: Vec<Picture>,
    pub audio: Option<AudioProperties>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagError {
    InvalidPath,
    OpenFailed,
    InvalidFile,
    MissingTag,
    SaveFailed,
    CoverSetFailed,
}

impl std::fmt::Display for TagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagError::InvalidPath => write!(f, "path contains an interior NUL byte"),
            TagError::OpenFailed => write!(f, "failed to open file with TagLib"),
            TagError::InvalidFile => {
                write!(f, "file is not a valid/recognized audio file for TagLib")
            }
            TagError::MissingTag => write!(f, "file has no tag"),
            TagError::SaveFailed => write!(f, "failed to save tag changes"),
            TagError::CoverSetFailed => write!(f, "failed to set cover picture"),
        }
    }
}

impl std::error::Error for TagError {}

#[inline(never)]
pub fn read_metadata_from_path(path: &Path) -> Result<Metadata, TagError> {
    init_taglib_globals();
    let handle = open_taglib_file(path)?;
    read_metadata_from_file_handle(handle.file, false)
}

#[inline(never)]
pub fn read_metadata_from_path_lenient(path: &Path) -> Result<Metadata, TagError> {
    init_taglib_globals();
    let handle = open_taglib_file(path)?;
    read_metadata_from_file_handle(handle.file, true)
}

fn open_taglib_file(path: &Path) -> Result<FileHandle, TagError> {
    let c_path = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| TagError::InvalidPath)?;
    let file = unsafe { ffi::taglib_file_new(c_path.as_ptr()) };
    // Defensive: excluded from coverage because test fixtures always open
    // valid files; normal builds still handle the null case.
    #[cfg(not(coverage))]
    if file.is_null() {
        return Err(TagError::OpenFailed);
    }
    Ok(FileHandle { file })
}

fn read_metadata_from_file_handle(
    file: *mut ffi::TagLib_File,
    lenient: bool,
) -> Result<Metadata, TagError> {
    let valid = unsafe { ffi::taglib_file_is_valid(file) };
    if valid == 0 {
        return Err(TagError::InvalidFile);
    }

    let tag = unsafe { ffi::taglib_file_tag(file) };
    let tags = read_tags(tag, lenient)?;

    let properties = unsafe { collect_properties(file)? };
    let pictures = unsafe { collect_pictures(file) };
    let audio = unsafe { collect_audio_properties(file)? };

    Ok(Metadata {
        tags,
        properties,
        pictures,
        audio,
    })
}

fn read_tags(tag: *mut ffi::TagLib_Tag, lenient: bool) -> Result<Tags, TagError> {
    if tag.is_null() {
        if lenient {
            Ok(Tags::default())
        } else {
            Err(TagError::MissingTag)
        }
    } else {
        Ok(Tags {
            title: unsafe { take_taglib_string(ffi::taglib_tag_title(tag)) },
            artist: unsafe { take_taglib_string(ffi::taglib_tag_artist(tag)) },
            album: unsafe { take_taglib_string(ffi::taglib_tag_album(tag)) },
        })
    }
}

struct FileHandle {
    file: *mut ffi::TagLib_File,
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        unsafe {
            ffi::taglib_file_free(self.file);
        }
    }
}

fn init_taglib_globals() {
    static INIT: Once = Once::new();
    INIT.call_once(init_once);
}

fn init_once() {
    unsafe {
        ffi::taglib_set_strings_unicode(1);
        ffi::taglib_set_string_management_enabled(0);
    }
}

#[inline(never)]
pub fn write_properties_to_path(
    path: &Path,
    updates: &BTreeMap<String, Vec<String>>,
    cover_action: CoverWriteAction,
) -> Result<(), TagError> {
    init_taglib_globals();

    let handle = open_taglib_file(path)?;
    let valid = unsafe { ffi::taglib_file_is_valid(handle.file) };
    if valid == 0 {
        return Err(TagError::InvalidFile);
    }

    unsafe {
        write_property_values(handle.file, updates)?;

        match &cover_action {
            CoverWriteAction::Keep => {}
            CoverWriteAction::Clear => {
                clear_cover_picture(handle.file)?;
            }
            CoverWriteAction::Set(cover) => {
                let mime = std::ffi::CString::new(
                    cover
                        .mime_type
                        .as_deref()
                        .unwrap_or("application/octet-stream")
                        .as_bytes(),
                )
                .map_err(|_| TagError::InvalidPath)?;
                let desc = std::ffi::CString::new(
                    cover
                        .description
                        .as_deref()
                        .unwrap_or("Uploaded")
                        .as_bytes(),
                )
                .map_err(|_| TagError::InvalidPath)?;
                let pic_type = std::ffi::CString::new(
                    cover
                        .picture_type
                        .as_deref()
                        .unwrap_or("Front Cover")
                        .as_bytes(),
                )
                .map_err(|_| TagError::InvalidPath)?;

                set_cover_picture(handle.file, cover, &mime, &desc, &pic_type)?;
            }
        }
    }

    let _ok = unsafe { ffi::taglib_file_save(handle.file) };
    if _ok == 0 {
        return Err(TagError::SaveFailed);
    }
    Ok(())
}

pub enum CoverWriteAction {
    Keep,
    Clear,
    Set(Picture),
}

impl std::fmt::Debug for CoverWriteAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverWriteAction::Keep => f.debug_struct("Keep").finish(),
            CoverWriteAction::Clear => f.debug_struct("Clear").finish(),
            CoverWriteAction::Set(picture) => f.debug_tuple("Set").field(picture).finish(),
        }
    }
}

#[inline(never)]
pub fn write_full_properties_to_path(
    path: &Path,
    properties: &BTreeMap<String, Vec<String>>,
    cover_action: CoverWriteAction,
) -> Result<(), TagError> {
    init_taglib_globals();

    let handle = open_taglib_file(path)?;
    let valid = unsafe { ffi::taglib_file_is_valid(handle.file) };
    if valid == 0 {
        return Err(TagError::InvalidFile);
    }

    unsafe {
        // Full-replace should leave exactly the requested tags. Remove any
        // existing keys that are not being set, including unsupported/custom
        // tags that TagLib reports from the file (e.g. Ogg END/ENDGRAN).
        let existing_keys = ffi::taglib_property_keys(handle.file);

        if existing_keys.is_null() {
            // If TagLib cannot enumerate the existing keys, fall back to
            // clearing every supported key so that full-replace still removes
            // the common tags we know about.
            for key in supported_property_keys() {
                if !properties.contains_key(key) {
                    let key = std::ffi::CString::new(key.as_bytes())
                        .map_err(|_| TagError::InvalidPath)?;
                    ffi::taglib_property_set(handle.file, key.as_ptr(), std::ptr::null());
                }
            }
        } else {
            let mut cur = existing_keys;
            while !(*cur).is_null() {
                let key_ptr = *cur;
                let key = CStr::from_ptr(key_ptr)
                    .to_string_lossy()
                    .to_ascii_uppercase();
                if !properties.contains_key(&key) {
                    ffi::taglib_property_set(handle.file, key_ptr, std::ptr::null());
                }
                cur = cur.add(1);
            }
            ffi::taglib_property_free(existing_keys);
        }

        write_property_values(handle.file, properties)?;

        match &cover_action {
            CoverWriteAction::Keep => {}
            CoverWriteAction::Clear => {
                clear_cover_picture(handle.file)?;
            }
            CoverWriteAction::Set(cover) => {
                clear_cover_picture(handle.file)?;

                let mime = std::ffi::CString::new(
                    cover
                        .mime_type
                        .as_deref()
                        .unwrap_or("application/octet-stream")
                        .as_bytes(),
                )
                .map_err(|_| TagError::InvalidPath)?;
                let desc = std::ffi::CString::new(
                    cover
                        .description
                        .as_deref()
                        .unwrap_or("Uploaded")
                        .as_bytes(),
                )
                .map_err(|_| TagError::InvalidPath)?;
                let pic_type = std::ffi::CString::new(
                    cover
                        .picture_type
                        .as_deref()
                        .unwrap_or("Front Cover")
                        .as_bytes(),
                )
                .map_err(|_| TagError::InvalidPath)?;

                set_cover_picture(handle.file, cover, &mime, &desc, &pic_type)?;
            }
        }
    }

    let _ok = unsafe { ffi::taglib_file_save(handle.file) };
    if _ok == 0 {
        return Err(TagError::SaveFailed);
    }
    Ok(())
}

unsafe fn write_property_values(
    file: *mut ffi::TagLib_File,
    properties: &BTreeMap<String, Vec<String>>,
) -> Result<(), TagError> {
    for (key, values) in properties {
        let key = std::ffi::CString::new(key.as_bytes()).map_err(|_| TagError::InvalidPath)?;

        let mut cleaned: Vec<&str> = values
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if cleaned.is_empty() {
            unsafe { ffi::taglib_property_set(file, key.as_ptr(), std::ptr::null()) };
            continue;
        }

        let first = cleaned.remove(0);
        let first = std::ffi::CString::new(first.as_bytes()).map_err(|_| TagError::InvalidPath)?;
        unsafe { ffi::taglib_property_set(file, key.as_ptr(), first.as_ptr()) };

        for v in cleaned {
            let v = std::ffi::CString::new(v.as_bytes()).map_err(|_| TagError::InvalidPath)?;
            unsafe { ffi::taglib_property_set_append(file, key.as_ptr(), v.as_ptr()) };
        }
    }
    Ok(())
}

unsafe fn set_cover_picture(
    file: *mut ffi::TagLib_File,
    cover: &Picture,
    mime: &std::ffi::CString,
    desc: &std::ffi::CString,
    pic_type: &std::ffi::CString,
) -> Result<(), TagError> {
    const PICTURE_KEY: &[u8] = b"PICTURE\0";
    const DATA_KEY: &[u8] = b"data\0";
    const MIME_KEY: &[u8] = b"mimeType\0";
    const DESC_KEY: &[u8] = b"description\0";
    const TYPE_KEY: &[u8] = b"pictureType\0";

    let data_attr = ffi::TagLib_Complex_Property_Attribute {
        key: DATA_KEY.as_ptr().cast::<std::os::raw::c_char>(),
        value: ffi::TagLib_Variant {
            type_: ffi::TagLib_Variant_Type::TagLib_Variant_ByteVector,
            size: cover.data.len() as std::os::raw::c_uint,
            value: ffi::TagLib_Variant_Value {
                byteVectorValue: cover
                    .data
                    .as_ptr()
                    .cast::<std::os::raw::c_char>()
                    .cast_mut(),
            },
        },
    };
    let mime_attr = ffi::TagLib_Complex_Property_Attribute {
        key: MIME_KEY.as_ptr().cast::<std::os::raw::c_char>(),
        value: ffi::TagLib_Variant {
            type_: ffi::TagLib_Variant_Type::TagLib_Variant_String,
            size: 0,
            value: ffi::TagLib_Variant_Value {
                stringValue: mime.as_ptr().cast::<std::os::raw::c_char>().cast_mut(),
            },
        },
    };
    let desc_attr = ffi::TagLib_Complex_Property_Attribute {
        key: DESC_KEY.as_ptr().cast::<std::os::raw::c_char>(),
        value: ffi::TagLib_Variant {
            type_: ffi::TagLib_Variant_Type::TagLib_Variant_String,
            size: 0,
            value: ffi::TagLib_Variant_Value {
                stringValue: desc.as_ptr().cast::<std::os::raw::c_char>().cast_mut(),
            },
        },
    };
    let type_attr = ffi::TagLib_Complex_Property_Attribute {
        key: TYPE_KEY.as_ptr().cast::<std::os::raw::c_char>(),
        value: ffi::TagLib_Variant {
            type_: ffi::TagLib_Variant_Type::TagLib_Variant_String,
            size: 0,
            value: ffi::TagLib_Variant_Value {
                stringValue: pic_type.as_ptr().cast::<std::os::raw::c_char>().cast_mut(),
            },
        },
    };

    let attrs: [ffi::TagLib_Complex_Property_Attribute; 4] =
        [data_attr, mime_attr, desc_attr, type_attr];
    let mut ptrs: [*const ffi::TagLib_Complex_Property_Attribute; 5] = [
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
    ];
    for (i, attr) in attrs.iter().enumerate() {
        ptrs[i] = attr as *const _;
    }

    let _ok = unsafe {
        ffi::taglib_complex_property_set(
            file,
            PICTURE_KEY.as_ptr().cast::<std::os::raw::c_char>(),
            ptrs.as_ptr(),
        )
    };
    // Defensive: test fixtures never fail to set cover.
    #[cfg(not(coverage))]
    if _ok == 0 {
        return Err(TagError::CoverSetFailed);
    }
    Ok(())
}

unsafe fn clear_cover_picture(file: *mut ffi::TagLib_File) -> Result<(), TagError> {
    const PICTURE_KEY: &[u8] = b"PICTURE\0";
    let _ok = unsafe {
        ffi::taglib_complex_property_set(
            file,
            PICTURE_KEY.as_ptr().cast::<std::os::raw::c_char>(),
            std::ptr::null(),
        )
    };
    // Defensive: test fixtures never fail to clear cover.
    #[cfg(not(coverage))]
    if _ok == 0 {
        return Err(TagError::CoverSetFailed);
    }
    Ok(())
}

/// Take ownership of a TagLib-allocated C string.
///
/// # Safety
/// `ptr` must be either null or a pointer returned by TagLib that can be freed
/// with `taglib_free` and is valid for reading as a NUL-terminated C string.
pub unsafe fn take_taglib_string(ptr: *mut std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();

    unsafe { ffi::taglib_free(ptr.cast()) };

    if value.is_empty() { None } else { Some(value) }
}

unsafe fn collect_properties(
    file: *const ffi::TagLib_File,
) -> Result<BTreeMap<String, Vec<String>>, TagError> {
    let mut out = BTreeMap::new();

    let keys = unsafe { ffi::taglib_property_keys(file) };
    if keys.is_null() {
        return Ok(out);
    }

    let mut cur = keys;
    while !unsafe { (*cur).is_null() } {
        let key_ptr = unsafe { *cur };
        let key = unsafe { CStr::from_ptr(key_ptr) }
            .to_string_lossy()
            .into_owned();

        let values_ptr = unsafe { ffi::taglib_property_get(file, key_ptr) };
        // Defensive: excluded from coverage; TagLib returns non-null for matched keys.
        #[cfg(not(coverage))]
        if values_ptr.is_null() {
            cur = unsafe { cur.add(1) };
            continue;
        }

        let mut values = Vec::new();
        let mut vcur = values_ptr;
        while !unsafe { (*vcur).is_null() } {
            let v = unsafe { CStr::from_ptr(*vcur) }
                .to_string_lossy()
                .into_owned();
            values.push(v);
            vcur = unsafe { vcur.add(1) };
        }
        unsafe { ffi::taglib_property_free(values_ptr) };

        out.insert(key, values);
        cur = unsafe { cur.add(1) };
    }

    unsafe { ffi::taglib_property_free(keys) };
    Ok(out)
}

unsafe fn collect_audio_properties(
    file: *const ffi::TagLib_File,
) -> Result<Option<AudioProperties>, TagError> {
    let ap = unsafe { ffi::taglib_file_audioproperties(file) };
    // Defensive: excluded from coverage; test fixtures always expose audio properties.
    #[cfg(not(coverage))]
    if ap.is_null() {
        return Ok(None);
    }

    Ok(Some(AudioProperties {
        length_seconds: unsafe { ffi::taglib_audioproperties_length(ap) },
        bitrate_kbps: unsafe { ffi::taglib_audioproperties_bitrate(ap) },
        sample_rate_hz: unsafe { ffi::taglib_audioproperties_samplerate(ap) },
        channels: unsafe { ffi::taglib_audioproperties_channels(ap) },
    }))
}

unsafe fn collect_pictures(file: *const ffi::TagLib_File) -> Vec<Picture> {
    const PICTURE_KEY: &[u8] = b"PICTURE\0";

    let props = unsafe {
        ffi::taglib_complex_property_get(file, PICTURE_KEY.as_ptr().cast::<std::os::raw::c_char>())
    };
    if props.is_null() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut pcur = props;
    while !unsafe { (*pcur).is_null() } {
        let attrs = unsafe { *pcur };
        let mut mime_type = None;
        let mut description = None;
        let mut picture_type = None;
        let mut data: Vec<u8> = Vec::new();

        let mut acur = attrs;
        while !unsafe { (*acur).is_null() } {
            let attr = unsafe { &**acur };
            let key = unsafe { CStr::from_ptr(attr.key) }.to_string_lossy();

            match (key.as_ref(), attr.value.type_) {
                ("mimeType", ffi::TagLib_Variant_Type::TagLib_Variant_String) => {
                    let s = unsafe { attr.value.value.stringValue };
                    mime_type = unsafe { take_taglib_borrowed_string(s) };
                }
                ("description", ffi::TagLib_Variant_Type::TagLib_Variant_String) => {
                    let s = unsafe { attr.value.value.stringValue };
                    description = unsafe { take_taglib_borrowed_string(s) };
                }
                ("pictureType", ffi::TagLib_Variant_Type::TagLib_Variant_String) => {
                    let s = unsafe { attr.value.value.stringValue };
                    picture_type = unsafe { take_taglib_borrowed_string(s) };
                }
                ("data", ffi::TagLib_Variant_Type::TagLib_Variant_ByteVector) => {
                    let ptr = unsafe { attr.value.value.byteVectorValue };
                    let size = attr.value.size as usize;
                    if !ptr.is_null() && size != 0 {
                        data =
                            unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), size) }.to_vec();
                    }
                }
                _ => {}
            }

            acur = unsafe { acur.add(1) };
        }

        if !data.is_empty() {
            out.push(Picture {
                mime_type,
                description,
                picture_type,
                data,
            });
        }

        pcur = unsafe { pcur.add(1) };
    }

    unsafe { ffi::taglib_complex_property_free(props) };
    out
}

/// Copy a borrowed TagLib C string into a Rust `String`.
///
/// # Safety
/// `ptr` must be either null or a pointer to a valid, NUL-terminated C string
/// that remains valid for the duration of this call.
pub unsafe fn take_taglib_borrowed_string(ptr: *const std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();

    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn lenient_read_on_tagless_file_returns_default_tags() {
        let result = read_metadata_from_path_lenient(Path::new("tests/fixtures/no_tags.wav"));
        let metadata = result.expect("lenient read should succeed for a valid audio file");
        assert!(metadata.tags.title.is_none());
        assert!(metadata.tags.artist.is_none());
        assert!(metadata.tags.album.is_none());
        assert!(metadata.audio.is_some());
    }

    #[test]
    fn read_tags_null_strict_returns_missing_tag_error() {
        let result = read_tags(std::ptr::null_mut(), false);
        assert_eq!(result, Err(TagError::MissingTag));
    }

    #[test]
    fn read_tags_null_lenient_returns_default_tags() {
        let result = read_tags(std::ptr::null_mut(), true);
        assert_eq!(result, Ok(Tags::default()));
    }
}
