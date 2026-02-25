use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainSession {
    pub cookies: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DomainSessionStore {
    sessions: Arc<RwLock<HashMap<String, DomainSession>>>,
}

impl DomainSessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, domain: &str) -> Option<DomainSession> {
        let sessions = self.sessions.read();
        sessions.get(domain).cloned()
    }

    pub fn has_session(&self, domain: &str) -> bool {
        let sessions = self.sessions.read();
        sessions.contains_key(domain)
    }

    pub fn set_cookies(&self, domain: &str, cookies: HashMap<String, String>) {
        let mut sessions = self.sessions.write();
        // extend existing cookies instead of replacing them.
        sessions
            .entry(domain.to_string())
            .or_default()
            .cookies
            .extend(cookies);
    }

    pub fn set_cookie(&self, domain: &str, name: &str, value: &str) {
        let mut sessions = self.sessions.write();
        sessions
            .entry(domain.to_string())
            .or_default()
            .cookies
            .insert(name.to_string(), value.to_string());
    }

    pub fn get_cookies(&self, domain: &str) -> HashMap<String, String> {
        let sessions = self.sessions.read();
        sessions
            .get(domain)
            .map(|s| s.cookies.clone())
            .unwrap_or_default()
    }

    pub fn set_headers(&self, domain: &str, headers: HashMap<String, String>) {
        let mut sessions = self.sessions.write();
        sessions.entry(domain.to_string()).or_default().headers = headers;
    }

    pub fn get_headers(&self, domain: &str) -> HashMap<String, String> {
        let sessions = self.sessions.read();
        sessions
            .get(domain)
            .map(|s| s.headers.clone())
            .unwrap_or_default()
    }

    pub fn set_user_agent(&self, domain: &str, user_agent: &str) {
        let mut sessions = self.sessions.write();
        sessions.entry(domain.to_string()).or_default().user_agent =
            Some(user_agent.to_string());
    }

    pub fn get_user_agent(&self, domain: &str) -> Option<String> {
        let sessions = self.sessions.read();
        sessions
            .get(domain)
            .and_then(|s| s.user_agent.clone())
    }

    pub fn remove_cookie(&self, domain: &str, name: &str) {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(domain) {
            session.cookies.remove(name);
        }
    }

    pub fn clear_cookies(&self, domain: &str) {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(domain) {
            session.cookies.clear();
        }
    }

    pub fn clear_all(&self) {
        let mut sessions = self.sessions.write();
        sessions.clear();
    }

    pub fn domains(&self) -> Vec<String> {
        let sessions = self.sessions.read();
        sessions.keys().cloned().collect()
    }

    pub fn export_all(&self) -> HashMap<String, DomainSession> {
        let sessions = self.sessions.read();
        sessions.clone()
    }

    pub fn import_all(&self, sessions: HashMap<String, DomainSession>) {
        let mut store = self.sessions.write();
        // merge imported sessions into existing ones instead of
        // overwriting whole DomainSession entries.
        for (domain, incoming) in sessions {
            let existing = store.entry(domain).or_default();
            existing.cookies.extend(incoming.cookies);
            existing.headers.extend(incoming.headers);
            if incoming.user_agent.is_some() {
                existing.user_agent = incoming.user_agent;
            }
        }
    }
}

impl Default for DomainSessionStore {
    fn default() -> Self {
        Self::new()
    }
}
