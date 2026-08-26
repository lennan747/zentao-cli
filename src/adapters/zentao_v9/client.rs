use std::sync::RwLock;
use std::time::Duration;

use reqwest::header::COOKIE;
use reqwest::{Client, Response};

use crate::domain::QueryError;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const SESSION_COOKIE: &str = "zentaosid";

/// 与禅道专业版 9.0.3 通信的 HTTP 客户端。
///
/// 旧版禅道会话只依赖单一 `zentaosid` Cookie（契约已确认），
/// 这里手动管理该 Cookie，避免引入整套 CookieStore 依赖。
#[derive(Clone)]
pub struct ZentaoV9Client {
    server: String,
    http: Client,
    cookie: std::sync::Arc<RwLock<Option<String>>>,
}

impl ZentaoV9Client {
    pub fn new(server: impl Into<String>) -> Result<Self, reqwest::Error> {
        Self::with_timeout(server, DEFAULT_TIMEOUT)
    }

    pub fn with_timeout(
        server: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            server: server.into(),
            http: Client::builder()
                .timeout(timeout)
                .connect_timeout(timeout)
                .build()?,
            cookie: std::sync::Arc::new(RwLock::new(None)),
        })
    }

    pub fn server(&self) -> &str {
        &self.server
    }

    pub fn http(&self) -> &Client {
        &self.http
    }

    /// GET 并返回响应文本。网络、超时和 HTTP 错误统一映射为 `QueryError::Remote`。
    pub async fn get_text(&self, url: &str) -> Result<String, QueryError> {
        let response = self.send_get(url).await?;
        let text = response
            .text()
            .await
            .map_err(|e| QueryError::Remote(format!("读取响应失败: {e}")))?;
        Ok(text)
    }

    /// 发送带会话 Cookie 的 GET 请求。
    pub async fn send_get(&self, url: &str) -> Result<Response, QueryError> {
        let request = self.http.get(url);
        self.send(request).await
    }

    /// 发送带会话 Cookie 的表单 POST 请求。
    pub async fn post_form(
        &self,
        url: &str,
        form: &[(&str, &str)],
    ) -> Result<Response, QueryError> {
        let request = self.http.post(url).form(form);
        self.send(request).await
    }

    /// 发送表单 POST 并返回响应文本。
    pub async fn post_form_text(
        &self,
        url: &str,
        form: &[(&str, &str)],
    ) -> Result<String, QueryError> {
        let response = self.post_form(url, form).await?;
        response
            .text()
            .await
            .map_err(|e| QueryError::Remote(format!("读取响应失败: {e}")))
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<Response, QueryError> {
        let request = if let Some(value) = self.session_cookie() {
            request.header(COOKIE, format!("{SESSION_COOKIE}={value}"))
        } else {
            request
        };
        let response = request
            .send()
            .await
            .map_err(|e| QueryError::Remote(format!("请求失败: {e}")))?;
        let response = response
            .error_for_status()
            .map_err(|e| QueryError::Remote(format!("HTTP 错误: {e}")))?;
        self.capture_session_cookie(&response);
        Ok(response)
    }

    /// 从响应 Set-Cookie 中捕获 `zentaosid` 会话值。
    pub fn capture_session_cookie(&self, response: &Response) {
        for cookie in response.cookies() {
            if cookie.name() == SESSION_COOKIE {
                if let Ok(mut slot) = self.cookie.write() {
                    *slot = Some(cookie.value().to_string());
                }
            }
        }
    }

    pub fn session_cookie(&self) -> Option<String> {
        self.cookie.read().ok().and_then(|c| c.clone())
    }

    /// 导出会话 Cookie 值（用于持久化）。
    pub fn export_cookies(&self) -> String {
        self.session_cookie().unwrap_or_default()
    }

    /// 导入会话 Cookie 值（用于恢复 session）。
    pub fn import_cookies(&self, cookie: &str) -> Result<(), QueryError> {
        if cookie.trim().is_empty() {
            return Err(QueryError::ParseError("会话 Cookie 为空".into()));
        }
        if let Ok(mut slot) = self.cookie.write() {
            *slot = Some(cookie.trim().to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_rejects_empty_cookie() {
        let client = ZentaoV9Client::new("https://x").unwrap();
        assert!(client.import_cookies("  ").is_err());
        client.import_cookies("abc").unwrap();
        assert_eq!(client.export_cookies(), "abc");
    }
}
