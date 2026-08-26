use async_trait::async_trait;

use crate::domain::{
    BugActivateParams, BugDetail, BugDraft, BugEdit, BugNoteParams, BugResolveParams, BugSummary,
    EntityId, Page, QueryError,
};

#[derive(Debug, Clone, Default)]
pub struct BugQuery {
    pub assigned_to: Option<String>,
    pub status: Option<String>,
    pub page: u64,
    pub per_page: u64,
}

/// Bug 查询与写操作端口。
#[async_trait]
pub trait BugGateway: Send + Sync {
    async fn list_bugs(&self, query: BugQuery) -> Result<Page<BugSummary>, QueryError>;
    async fn get_bug(&self, id: EntityId) -> Result<BugDetail, QueryError>;

    async fn create_bug(&self, product: EntityId, draft: BugDraft) -> Result<(), QueryError>;
    async fn edit_bug(&self, id: EntityId, edit: BugEdit) -> Result<(), QueryError>;
    async fn resolve_bug(&self, id: EntityId, params: BugResolveParams) -> Result<(), QueryError>;
    async fn activate_bug(&self, id: EntityId, params: BugActivateParams) -> Result<(), QueryError>;
    async fn close_bug(&self, id: EntityId, params: BugNoteParams) -> Result<(), QueryError>;
    async fn confirm_bug(&self, id: EntityId, params: BugNoteParams) -> Result<(), QueryError>;
    async fn comment_bug(&self, id: EntityId, comment: &str) -> Result<(), QueryError>;
}
