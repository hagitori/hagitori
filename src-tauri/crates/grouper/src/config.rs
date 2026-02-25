use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupFormat {
    /// Comic Book Zip archive (`.cbz`).
    Cbz,
    /// Standard ZIP archive (`.zip`).
    Zip,
}
