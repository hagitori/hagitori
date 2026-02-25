//! extension sync: catalog, installation, updates, and auto-update.

pub mod auto_update;
pub mod catalog;
pub mod installer;
pub mod integrity;
pub mod updater;

pub use auto_update::{run_auto_update, AutoUpdateResult, AutoUpdatedEntry, AutoUpdateFailure};
pub use catalog::CatalogFetcher;
pub use installer::ExtensionInstaller;
pub use updater::UpdateChecker;
