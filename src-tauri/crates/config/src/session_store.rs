use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::debug;

use hagitori_core::error::{HagitoriError, Result};

use crate::database;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionData {
    pub cookies: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub user_agent: Option<String>,
}

pub struct SessionStore {
    conn: Mutex<rusqlite::Connection>,
}

impl SessionStore {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let conn = database::open_sessions_db(base_dir)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// persists session data for the given `domain`, overwriting any previous entry.
    pub fn save(&self, domain: &str, session: &SessionData) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;

        let cookies_json = serde_json::to_string(&session.cookies)
            .map_err(|e| HagitoriError::config(format!("failed to serialize cookies: {e}")))?;
        let headers_json = serde_json::to_string(&session.headers)
            .map_err(|e| HagitoriError::config(format!("failed to serialize headers: {e}")))?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO sessions (domain, cookies, headers, user_agent, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![domain, cookies_json, headers_json, session.user_agent, now],
        )
        .map_err(|e| HagitoriError::config(format!("failed to save session for '{domain}': {e}")))?;

        debug!(domain = domain, "SessionStore::save");
        Ok(())
    }

    pub fn load_all(&self) -> Result<HashMap<String, SessionData>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT domain, cookies, headers, user_agent FROM sessions")
            .map_err(|e| HagitoriError::config(format!("failed to prepare query: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let domain: String = row.get(0)?;
                let cookies_json: Option<String> = row.get(1)?;
                let headers_json: Option<String> = row.get(2)?;
                let user_agent: Option<String> = row.get(3)?;
                Ok((domain, cookies_json, headers_json, user_agent))
            })
            .map_err(|e| HagitoriError::config(format!("failed to read sessions: {e}")))?;

        let mut map = HashMap::new();
        for row in rows {
            let (domain, cookies_json, headers_json, user_agent) =
                row.map_err(|e| HagitoriError::config(format!("failed to read row: {e}")))?;

            let cookies: HashMap<String, String> = match cookies_json.as_deref() {
                Some(j) => serde_json::from_str(j).map_err(|e| {
                    HagitoriError::config(format!(
                        "corrupted session cookies for '{domain}': {e}"
                    ))
                })?,
                None => HashMap::new(),
            };
            let headers: HashMap<String, String> = match headers_json.as_deref() {
                Some(j) => serde_json::from_str(j).map_err(|e| {
                    HagitoriError::config(format!(
                        "corrupted session headers for '{domain}': {e}"
                    ))
                })?,
                None => HashMap::new(),
            };

            map.insert(
                domain,
                SessionData {
                    cookies,
                    headers,
                    user_agent,
                },
            );
        }

        Ok(map)
    }
}
