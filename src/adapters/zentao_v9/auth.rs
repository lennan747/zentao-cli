use async_trait::async_trait;
use md5::{Digest, Md5};

use crate::application::{AuthGateway, Credentials, Session};
use crate::domain::AuthError;

use super::client::ZentaoV9Client;
use super::response::parse_body;
use super::routes::Routes;

fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn remote_error(e: crate::domain::QueryError) -> AuthError {
    AuthError::Other(e.to_string())
}

/// 禅道 V9 认证网关实现。
///
/// 登录流程（契约见 source-material/zentao-cli/api-contracts/login-form.md）：
/// 1. `GET /user-refreshRandom.html` 获取纯数字 `verifyRand`；
/// 2. 计算 `MD5(MD5(密码) + verifyRand)`；
/// 3. `POST /user-login.html` 提交表单；
/// 4. 用已确认的只读端点 `/my-task.json` 探测会话是否建立。
pub struct ZentaoV9AuthGateway {
    client: ZentaoV9Client,
}

impl ZentaoV9AuthGateway {
    pub fn new(client: ZentaoV9Client) -> Self {
        Self { client }
    }

    async fn probe_session(&self, server: &str) -> Result<(), AuthError> {
        let body = self
            .client
            .get_text(&Routes::my_task(server))
            .await
            .map_err(remote_error)?;
        match parse_body(&body) {
            Ok(_) => Ok(()),
            Err(crate::domain::QueryError::SessionExpired) => Err(AuthError::InvalidCredentials),
            Err(e) => Err(AuthError::Other(e.to_string())),
        }
    }
}

#[async_trait]
impl AuthGateway for ZentaoV9AuthGateway {
    async fn login(&self, credentials: &Credentials) -> Result<Session, AuthError> {
        let server = self.client.server().to_string();

        // verifyRand 与 zentaosid 会话绑定：先访问登录页建立会话，否则提交时
        // 服务端会按新会话校验随机数，导致登录失败。
        self.client
            .send_get(&Routes::login_page(&server))
            .await
            .map_err(remote_error)?;

        let rand_body = self
            .client
            .get_text(&Routes::refresh_random(&server))
            .await
            .map_err(remote_error)?;
        let verify_rand = rand_body.trim();
        if verify_rand.is_empty() || !verify_rand.bytes().all(|b| b.is_ascii_digit()) {
            return Err(AuthError::Other("获取 verifyRand 失败".into()));
        }

        let password_hash = md5_hex(&format!(
            "{}{}",
            md5_hex(&credentials.password),
            verify_rand
        ));

        self.client
            .post_form(
                &Routes::login(&server),
                &[
                    ("account", credentials.account.as_str()),
                    ("password", password_hash.as_str()),
                    ("passwordStrength", "1"),
                    ("referer", "/"),
                    ("verifyRand", verify_rand),
                    ("keepLogin", "0"),
                ],
            )
            .await
            .map_err(remote_error)?;

        // 旧版禅道登录失败也返回 HTTP 200，必须用只读端点验证会话。
        self.probe_session(&server).await?;

        Ok(Session {
            server,
            cookie: self.client.export_cookies(),
        })
    }

    async fn validate(&self, session: &Session) -> Result<(), AuthError> {
        self.client
            .import_cookies(&session.cookie)
            .map_err(remote_error)?;

        let body = self
            .client
            .get_text(&Routes::my_task(&session.server))
            .await
            .map_err(remote_error)?;
        match parse_body(&body) {
            Ok(_) => Ok(()),
            Err(crate::domain::QueryError::SessionExpired) => Err(AuthError::SessionExpired),
            Err(e) => Err(AuthError::Other(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_md5_uses_lowercase_hex() {
        let inner = md5_hex("secret");
        let outer = md5_hex(&format!("{inner}123456"));
        assert_eq!(outer.len(), 32);
        assert!(outer
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
