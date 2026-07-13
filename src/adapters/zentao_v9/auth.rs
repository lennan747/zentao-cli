use async_trait::async_trait;

use crate::application::{AuthGateway, Credentials, Session};
use crate::domain::AuthError;

use super::client::ZentaoV9Client;

/// 禅道 V9 认证网关实现。
///
/// 当前为骨架实现：端口已定义，具体登录逻辑（verifyRand、双 MD5、表单提交）在子任务 03 实现。
pub struct ZentaoV9AuthGateway {
    client: ZentaoV9Client,
}

impl ZentaoV9AuthGateway {
    pub fn new(client: ZentaoV9Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AuthGateway for ZentaoV9AuthGateway {
    async fn login(&self, _credentials: &Credentials) -> Result<Session, AuthError> {
        // 子任务 03 实现真实登录。
        Err(AuthError::Other(
            "login not implemented in skeleton".into(),
        ))
    }

    async fn validate(&self, _session: &Session) -> Result<(), AuthError> {
        // 子任务 03 实现。
        Err(AuthError::Other(
            "validate not implemented in skeleton".into(),
        ))
    }
}
