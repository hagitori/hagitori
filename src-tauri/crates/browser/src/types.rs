use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InterceptedRequest {
    pub url: String,
    pub method: String,
    pub post_body: Option<String>,
    pub headers: HashMap<String, String>,
    pub resource_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InterceptedResponse {
    pub url: String,
    pub status: u16,
    pub body: String,
    pub base64_encoded: bool,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InterceptedPageData {
    pub requests: Vec<InterceptedRequest>,
    pub responses: Vec<InterceptedResponse>,
}
