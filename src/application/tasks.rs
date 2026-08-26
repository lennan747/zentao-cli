use async_trait::async_trait;

use crate::domain::{
    EntityId, Page, QueryError, TaskDetail, TaskDraft, TaskEdit, TaskFinishParams, TaskNoteParams,
    TaskStartParams, TaskSummary,
};

#[derive(Debug, Clone, Default)]
pub struct TaskQuery {
    pub assigned_to: Option<String>,
    pub status: Option<String>,
    pub page: u64,
    pub per_page: u64,
}

/// 任务查询与写操作端口。
#[async_trait]
pub trait TaskGateway: Send + Sync {
    async fn list_tasks(&self, query: TaskQuery) -> Result<Page<TaskSummary>, QueryError>;
    async fn get_task(&self, id: EntityId) -> Result<TaskDetail, QueryError>;

    async fn create_task(&self, project: EntityId, draft: TaskDraft) -> Result<(), QueryError>;
    async fn edit_task(&self, id: EntityId, edit: TaskEdit) -> Result<(), QueryError>;
    async fn start_task(&self, id: EntityId, params: TaskStartParams) -> Result<(), QueryError>;
    async fn finish_task(&self, id: EntityId, params: TaskFinishParams) -> Result<(), QueryError>;
    async fn cancel_task(&self, id: EntityId, params: TaskNoteParams) -> Result<(), QueryError>;
    async fn close_task(&self, id: EntityId, params: TaskNoteParams) -> Result<(), QueryError>;
    async fn activate_task(&self, id: EntityId, params: TaskNoteParams) -> Result<(), QueryError>;
    async fn comment_task(&self, id: EntityId, comment: &str) -> Result<(), QueryError>;
}
