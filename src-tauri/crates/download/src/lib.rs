//! # hagitori-download
//!
//! concurrent manga chapter page download engine.

mod browser;
pub mod engine;
mod http;
mod image;

pub use engine::{DownloadEngine, DownloadEngineConfig};
