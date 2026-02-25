use std::collections::HashMap;

use hagitori_http::{DomainSessionStore, HttpClient};

#[test]
fn session_store_cookies() {
    let store = DomainSessionStore::new();

    // batch set
    let mut cookies = HashMap::new();
    cookies.insert("cf_clearance".to_string(), "abc123".to_string());
    cookies.insert("session_id".to_string(), "xyz789".to_string());
    store.set_cookies("mangasite.com", cookies.clone());
    assert_eq!(store.get_cookies("mangasite.com"), cookies);

    // individual set
    store.set_cookie("other.com", "token", "xyz");
    assert_eq!(store.get_cookies("other.com").get("token").unwrap(), "xyz");
}

#[test]
fn session_store_empty_domain_returns_empty() {
    let store = DomainSessionStore::new();
    assert!(store.get_cookies("unknown.com").is_empty());
    assert!(store.get_headers("unknown.com").is_empty());
    assert!(store.get_user_agent("unknown.com").is_none());
}

#[test]
fn session_store_headers_and_user_agent() {
    let store = DomainSessionStore::new();

    let mut headers = HashMap::new();
    headers.insert("Referer".to_string(), "https://mangasite.com".to_string());
    store.set_headers("mangasite.com", headers);
    assert_eq!(store.get_headers("mangasite.com").get("Referer").unwrap(), "https://mangasite.com");

    store.set_user_agent("mangasite.com", "Custom UA");
    assert_eq!(store.get_user_agent("mangasite.com"), Some("Custom UA".to_string()));
}

#[test]
fn session_store_domains_and_clear() {
    let store = DomainSessionStore::new();
    store.set_cookie("a.com", "k", "v");
    store.set_cookie("b.com", "k", "v");
    store.set_cookie("c.com", "k", "v");

    let mut domains = store.domains();
    domains.sort();
    assert_eq!(domains, vec!["a.com", "b.com", "c.com"]);

    store.clear_all();
    assert!(store.domains().is_empty());
}

#[test]
fn session_store_export_import() {
    let store1 = DomainSessionStore::new();
    store1.set_cookie("site.com", "cf", "abc");
    let mut headers = HashMap::new();
    headers.insert("Referer".to_string(), "https://site.com".to_string());
    store1.set_headers("site.com", headers);
    store1.set_user_agent("site.com", "Custom UA");

    let exported = store1.export_all();
    let store2 = DomainSessionStore::new();
    store2.import_all(exported);

    assert_eq!(store2.get_cookies("site.com").get("cf").unwrap(), "abc");
    assert_eq!(store2.get_headers("site.com").get("Referer").unwrap(), "https://site.com");
    assert_eq!(store2.get_user_agent("site.com"), Some("Custom UA".to_string()));
}

#[test]
fn session_store_is_thread_safe() {
    use std::sync::Arc;
    use std::thread;

    let store = Arc::new(DomainSessionStore::new());
    let mut handles = vec![];

    for i in 0..10 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            store_clone.set_cookie(&format!("site{i}.com"), "key", &format!("val{i}"));
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(store.domains().len(), 10);
}

#[test]
fn http_client_exposes_session_store() {
    let client = HttpClient::new().unwrap();
    client.session_store().set_cookie("test.com", "k", "v");
    assert_eq!(client.session_store().get_cookies("test.com").get("k").unwrap(), "v");
}

#[tokio::test]
async fn http_client_invalid_url_returns_error() {
    let client = HttpClient::new().unwrap();
    assert!(client.get("not-a-valid-url", None).await.is_err());
}
