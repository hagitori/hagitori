//! # hagitori-core
//!
//! core types (entities, errors and prelude) for the Hagitori manga downloader.

pub mod entities;
pub mod error;
pub mod prelude;
pub mod provider;

pub use error::{HagitoriError, Result};
