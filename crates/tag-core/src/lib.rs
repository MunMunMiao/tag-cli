#![allow(unexpected_cfgs)]

pub mod config;
pub mod error;
pub mod image_proc;
pub mod output;
pub mod workflow;

pub mod taglib {
    pub use taglib_rs::*;
}

pub use error::TagCliError;

#[cfg(test)]
pub mod test_helpers;
