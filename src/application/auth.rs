use async_trait::async_trait;

use crate::domain::AuthError;

/// 用户凭据。密码只在登录流程中使用，不持久化。
#[derive(Debug, Clone)]
pub struct Credentials {
    pub account: String,
    pub password: String,
}

/// 已登录会话。只保存 server origin 和 session cookie，不保存密码。
#[derive(Debug, Clone)]
pub struct Session {
    pub server: String,
    pub cookie: String,
}

/// 认证端口。
#[async_trait]
pub trait AuthGateway: Send + Sync {
    async fn login(&self, credentials: &Credentials) -> Result<Session, AuthError>;
    async fn validate(&self, session: &Session) -> Result<(), AuthError>;
}
