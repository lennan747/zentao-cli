use async_trait::async_trait;

use crate::domain::{BugDetail, BugSummary, EntityId, Page, QueryError};

#[derive(Debug, Clone, Default)]
pub struct BugQuery {
    pub assigned_to: Option<String>,
    pub status: Option<String>,
    pub page: u64,
    pub per_page: u64,
}

/// Bug 查询端口。
#[async_trait]
pub trait BugGateway: Send + Sync {
    async fn list_bugs(&self, query: BugQuery) -> Result<Page<BugSummary>, QueryError>;
    async fn get_bug(&self, id: EntityId) -> Result<BugDetail, QueryError>;
}
