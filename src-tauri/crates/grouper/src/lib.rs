//! # hagitori-grouper
//!
//! CBZ packaging and ComicInfo.xml metadata generation.

pub mod cbz;
pub mod cleanup;
pub mod config;
pub mod metadata;

pub use cbz::create_archive;
pub use cleanup::cleanup_chapter;
pub use config::GroupFormat;
pub use metadata::ComicInfo;
