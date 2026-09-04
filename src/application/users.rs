use async_trait::async_trait;

use crate::domain::{QueryError, UserSummary};

/// 用户查询端口（账号 → 真实姓名）。
#[async_trait]
pub trait UserGateway: Send + Sync {
    /// 全量用户列表。旧版接口无分页契约，一次返回；失败时返回最后一个来源的错误。
    async fn list_users(&self) -> Result<Vec<UserSummary>, QueryError>;
}
