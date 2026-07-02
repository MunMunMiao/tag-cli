#![allow(unexpected_cfgs)]

mod ffi;
mod keys;
mod wrapper;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use keys::{supported_property_keys, supported_property_keys_set};
pub use wrapper::{
    AudioProperties, CoverWriteAction, Metadata, Picture, TagError, Tags, read_metadata_from_path,
    read_metadata_from_path_lenient, write_full_properties_to_path, write_properties_to_path,
};

#[cfg(any(test, feature = "test-utils"))]
pub use wrapper::{take_taglib_borrowed_string, take_taglib_string};
