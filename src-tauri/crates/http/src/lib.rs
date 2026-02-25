//! # hagitori-http
//!
//! HTTP client with automatic retry, user-agent rotation, and per-domain sessions.

pub mod client;
pub mod session_store;

pub use client::HttpClient;
pub use client::RequestOptions;
pub use session_store::{DomainSession, DomainSessionStore};
