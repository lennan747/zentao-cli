use async_trait::async_trait;

use crate::domain::{EntityId, Page, QueryError, TaskDetail, TaskSummary};

#[derive(Debug, Clone, Default)]
pub struct TaskQuery {
    pub assigned_to: Option<String>,
    pub status: Option<String>,
    pub page: u64,
    pub per_page: u64,
}

/// 任务查询端口。
#[async_trait]
pub trait TaskGateway: Send + Sync {
    async fn list_tasks(&self, query: TaskQuery) -> Result<Page<TaskSummary>, QueryError>;
    async fn get_task(&self, id: EntityId) -> Result<TaskDetail, QueryError>;
}
