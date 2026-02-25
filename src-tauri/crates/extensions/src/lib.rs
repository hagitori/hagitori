//! # hagitori-extensions
//!
//! rquickjs JavaScript runtime for executing manga provider extensions.

pub mod extension;
pub mod loader;
pub mod manifest;
pub mod runtime;
pub mod apis;

pub use extension::JsExtension;
pub use loader::ExtensionLoader;
pub use manifest::{ExtensionManifest, CURRENT_API_VERSION};
pub use runtime::JsRuntime;
