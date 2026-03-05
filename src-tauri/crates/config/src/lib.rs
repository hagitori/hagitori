//! # hagitori-config
//!
//! application config, SQLite persistence and extension registry.

pub mod config_manager;
pub mod database;
pub mod extension_registry;
pub mod history;
pub mod library_manager;
pub mod session_store;

pub use config_manager::ConfigManager;
pub use database::data_dir;
pub use extension_registry::ExtensionRegistry;
pub use history::{DownloadHistory, DownloadRecord};
pub use library_manager::{LibraryEntry, LibraryManager, SourceMeta};
pub use session_store::{SessionData, SessionStore};
