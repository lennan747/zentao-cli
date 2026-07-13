use std::sync::Arc;

use cookie_store::CookieStore;
use reqwest::Client;
use reqwest_cookie_store::{CookieStoreMutex};

/// 与禅道专业版 9.0.3 通信的 HTTP 客户端。
#[derive(Clone)]
pub struct ZentaoV9Client {
    server: String,
    http: Client,
    cookie_store: Arc<CookieStoreMutex>,
}

impl ZentaoV9Client {
    pub fn new(server: impl Into<String>) -> Result<Self, reqwest::Error> {
        let server = server.into();
        let cookie_store = CookieStore::new(None);
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));

        let http = Client::builder()
            .cookie_provider(Arc::clone(&cookie_store) as Arc<_>)
            .cookie_store(true)
            .build()?;

        Ok(Self {
            server,
            http,
            cookie_store,
        })
    }

    pub fn server(&self) -> &str {
        &self.server
    }

    pub fn http(&self) -> &Client {
        &self.http
    }

    /// 导出当前 Cookie 字符串（用于持久化 session）。
    pub fn export_cookies(&self) -> String {
        let store = self.cookie_store.lock().unwrap_or_else(|e| e.into_inner());
        serde_json::to_string(&*store).unwrap_or_default()
    }

    /// 导入 Cookie 字符串（用于恢复 session）。
    pub fn import_cookies(&self, json: &str) -> Result<(), crate::domain::QueryError> {
        let store: CookieStore = serde_json::from_str(json)
            .map_err(|e| crate::domain::QueryError::ParseError(format!("cookie parse: {e}")))?;
        *self.cookie_store.lock().unwrap_or_else(|e| e.into_inner()) = store;
        Ok(())
    }
}
