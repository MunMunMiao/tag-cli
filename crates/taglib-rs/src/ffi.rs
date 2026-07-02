#![allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::enum_variant_names,
    clippy::upper_case_acronyms
)]

use std::os::raw::{c_char, c_int, c_uint, c_void};

#[repr(C)]
pub struct TagLib_File {
    _private: [u8; 0],
}

#[repr(C)]
pub struct TagLib_Tag {
    _private: [u8; 0],
}

#[repr(C)]
pub struct TagLib_AudioProperties {
    _private: [u8; 0],
}

#[repr(C)]
pub struct TagLib_IOStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct TagLib_Complex_Property_Attribute {
    pub key: *const c_char,
    pub value: TagLib_Variant,
}

#[repr(C)]
pub union TagLib_Variant_Value {
    pub stringValue: *mut c_char,
    pub stringListValue: *mut *mut c_char,
    pub byteVectorValue: *mut c_char,
    pub byteVectorListValue: *mut *mut *mut c_char,
    pub boolValue: c_int,
    pub intValue: c_int,
    pub uintValue: c_uint,
    pub longLongValue: i64,
    pub ulongLongValue: u64,
}

#[repr(C)]
pub struct TagLib_Variant {
    pub type_: TagLib_Variant_Type,
    pub size: c_uint,
    pub value: TagLib_Variant_Value,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TagLib_Variant_Type {
    TagLib_Variant_Void,
    TagLib_Variant_Bool,
    TagLib_Variant_Int,
    TagLib_Variant_UInt,
    TagLib_Variant_LongLong,
    TagLib_Variant_ULongLong,
    TagLib_Variant_Double,
    TagLib_Variant_String,
    TagLib_Variant_StringList,
    TagLib_Variant_ByteVector,
    TagLib_Variant_ByteVectorList,
}

#[repr(C)]
pub struct TagLib_Complex_Property_Picture_Data {
    pub data: *const c_char,
    pub size: c_uint,
    pub mimeType: *const c_char,
    pub description: *const c_char,
    pub pictureType: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TagLib_ID3v2_Encoding {
    TagLib_ID3v2_Latin1 = 0,
    TagLib_ID3v2_UTF16 = 1,
    TagLib_ID3v2_UTF16BE = 2,
    TagLib_ID3v2_UTF8 = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TagLib_File_Type {
    TagLib_File_MPEG = 0,
    TagLib_File_OggVorbis = 1,
    TagLib_File_FLAC = 2,
    TagLib_File_MPC = 3,
    TagLib_File_OggFlac = 4,
    TagLib_File_WavPack = 5,
    TagLib_File_Speex = 6,
    TagLib_File_TrueAudio = 7,
    TagLib_File_MP4 = 8,
    TagLib_File_ASF = 9,
}

pub type BOOL = c_int;

unsafe extern "C" {
    pub fn taglib_set_strings_unicode(unicode: BOOL);
    pub fn taglib_set_string_management_enabled(management: BOOL);
    pub fn taglib_free(pointer: *mut c_void);
    pub fn taglib_tag_free_strings();

    pub fn taglib_file_new(filename: *const c_char) -> *mut TagLib_File;
    pub fn taglib_file_new_type(
        filename: *const c_char,
        file_type: TagLib_File_Type,
    ) -> *mut TagLib_File;
    pub fn taglib_file_free(file: *mut TagLib_File);
    pub fn taglib_file_is_valid(file: *const TagLib_File) -> BOOL;
    pub fn taglib_file_tag(file: *const TagLib_File) -> *mut TagLib_Tag;
    pub fn taglib_file_audioproperties(file: *const TagLib_File) -> *const TagLib_AudioProperties;
    pub fn taglib_file_save(file: *mut TagLib_File) -> BOOL;

    pub fn taglib_tag_title(tag: *const TagLib_Tag) -> *mut c_char;
    pub fn taglib_tag_artist(tag: *const TagLib_Tag) -> *mut c_char;
    pub fn taglib_tag_album(tag: *const TagLib_Tag) -> *mut c_char;
    pub fn taglib_tag_comment(tag: *const TagLib_Tag) -> *mut c_char;
    pub fn taglib_tag_genre(tag: *const TagLib_Tag) -> *mut c_char;
    pub fn taglib_tag_year(tag: *const TagLib_Tag) -> c_uint;
    pub fn taglib_tag_track(tag: *const TagLib_Tag) -> c_uint;
    pub fn taglib_tag_set_title(tag: *mut TagLib_Tag, title: *const c_char);
    pub fn taglib_tag_set_artist(tag: *mut TagLib_Tag, artist: *const c_char);
    pub fn taglib_tag_set_album(tag: *mut TagLib_Tag, album: *const c_char);
    pub fn taglib_tag_set_comment(tag: *mut TagLib_Tag, comment: *const c_char);
    pub fn taglib_tag_set_genre(tag: *mut TagLib_Tag, genre: *const c_char);
    pub fn taglib_tag_set_year(tag: *mut TagLib_Tag, year: c_uint);
    pub fn taglib_tag_set_track(tag: *mut TagLib_Tag, track: c_uint);

    pub fn taglib_audioproperties_length(audioProperties: *const TagLib_AudioProperties) -> c_int;
    pub fn taglib_audioproperties_bitrate(audioProperties: *const TagLib_AudioProperties) -> c_int;
    pub fn taglib_audioproperties_samplerate(
        audioProperties: *const TagLib_AudioProperties,
    ) -> c_int;
    pub fn taglib_audioproperties_channels(audioProperties: *const TagLib_AudioProperties)
    -> c_int;

    pub fn taglib_property_set(file: *mut TagLib_File, prop: *const c_char, value: *const c_char);
    pub fn taglib_property_set_append(
        file: *mut TagLib_File,
        prop: *const c_char,
        value: *const c_char,
    );
    pub fn taglib_property_keys(file: *const TagLib_File) -> *mut *mut c_char;
    pub fn taglib_property_get(file: *const TagLib_File, prop: *const c_char) -> *mut *mut c_char;
    pub fn taglib_property_free(props: *mut *mut c_char);

    pub fn taglib_complex_property_set(
        file: *mut TagLib_File,
        key: *const c_char,
        value: *const *const TagLib_Complex_Property_Attribute,
    ) -> BOOL;
    pub fn taglib_complex_property_set_append(
        file: *mut TagLib_File,
        key: *const c_char,
        value: *const *const TagLib_Complex_Property_Attribute,
    ) -> BOOL;
    pub fn taglib_complex_property_keys(file: *const TagLib_File) -> *mut *mut c_char;
    pub fn taglib_complex_property_get(
        file: *const TagLib_File,
        key: *const c_char,
    ) -> *mut *mut *mut TagLib_Complex_Property_Attribute;
    pub fn taglib_picture_from_complex_property(
        properties: *mut *mut *mut TagLib_Complex_Property_Attribute,
        picture: *mut TagLib_Complex_Property_Picture_Data,
    );
    pub fn taglib_complex_property_free_keys(keys: *mut *mut c_char);
    pub fn taglib_complex_property_free(props: *mut *mut *mut TagLib_Complex_Property_Attribute);

    pub fn taglib_id3v2_set_default_text_encoding(encoding: TagLib_ID3v2_Encoding);
}
