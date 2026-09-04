use async_trait::async_trait;
use serde_json::Value;

use crate::application::UserGateway;
use crate::domain::{QueryError, UserSummary};

use super::client::ZentaoV9Client;
use super::response::parse_body;
use super::routes::Routes;

/// 用户查询网关（禅道 V9 旧版 `.json` 接口）。
///
/// 契约探索（2026-09-04）：旧版几乎所有页面响应顶层都附带全量 `users` 映射
/// （account → realname）；`user-index.json` 仅管理员可访问。主源取
/// `my-task.json`（响应最轻），备源 `project-all-0.json`，两者与详情页一致。
pub struct ZentaoV9UserGateway {
    client: ZentaoV9Client,
}

impl ZentaoV9UserGateway {
    pub fn new(client: ZentaoV9Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl UserGateway for ZentaoV9UserGateway {
    async fn list_users(&self) -> Result<Vec<UserSummary>, QueryError> {
        let server = self.client.server();
        let sources = [Routes::my_task(server), Routes::project_all(server, "0")];
        let mut last_err = QueryError::IncompatibleResponse;
        for url in sources {
            match fetch_users(&self.client, &url).await {
                Ok(users) if !users.is_empty() => return Ok(users),
                Ok(_) => {
                    tracing::debug!(url = %url, "响应缺少 users 映射，尝试下一来源");
                    last_err = QueryError::IncompatibleResponse;
                }
                // 会话过期对所有来源一致，立即返回以触发自动重登。
                Err(e @ QueryError::SessionExpired) => return Err(e),
                Err(e) => {
                    tracing::debug!(url = %url, error = %e, "获取用户列表失败，尝试下一来源");
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }
}

async fn fetch_users(client: &ZentaoV9Client, url: &str) -> Result<Vec<UserSummary>, QueryError> {
    let body = client.get_text(url).await?;
    let data = parse_body(&body)?;
    Ok(parse_users(data.get("users")))
}

/// 解析顶层 `users`：兼容对象（account → realname）与数组（含 account/realname 字段）两种形态。
pub(super) fn parse_users(value: Option<&Value>) -> Vec<UserSummary> {
    match value {
        Some(Value::Object(map)) => map
            .iter()
            .filter(|(account, _)| !account.is_empty())
            .map(|(account, realname)| UserSummary {
                account: account.clone(),
                realname: realname.as_str().unwrap_or_default().to_string(),
            })
            .collect(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                let account = item.get("account")?.as_str()?.to_string();
                if account.is_empty() {
                    return None;
                }
                let realname = item
                    .get("realname")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                Some(UserSummary { account, realname })
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_object_map_form() {
        let value = json!({"zhangsan": "张三", "admin": ""});
        let users = parse_users(Some(&value));
        assert_eq!(users.len(), 2);
        // serde_json 对象按键排序迭代。
        assert_eq!(users[0].account, "admin");
        assert_eq!(users[0].realname, "");
        assert_eq!(users[1].account, "zhangsan");
        assert_eq!(users[1].realname, "张三");
    }

    #[test]
    fn parses_array_form() {
        let value = json!([
            {"account": "zhangsan", "realname": "张三"},
            {"account": "lisi"},
            {"realname": "无账号"},
            {"account": "", "realname": "空账号"}
        ]);
        let users = parse_users(Some(&value));
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].account, "zhangsan");
        assert_eq!(users[1].account, "lisi");
        assert_eq!(users[1].realname, "");
    }

    #[test]
    fn missing_or_invalid_users_yields_empty() {
        assert!(parse_users(None).is_empty());
        assert!(parse_users(Some(&json!("str"))).is_empty());
        assert!(parse_users(Some(&json!({}))).is_empty());
    }
}
