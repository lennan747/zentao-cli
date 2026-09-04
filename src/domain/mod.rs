pub mod bug;
pub mod error;
pub mod project;
pub mod task;
pub mod user;

#[allow(unused_imports)]
pub use bug::{
    BugActivateParams, BugDetail, BugDraft, BugEdit, BugNoteParams, BugResolveParams, BugSeverity,
    BugStatus, BugSummary, FieldChange, HistoryEntry,
};
pub use error::{AuthError, QueryError, ZentaoError};
#[allow(unused_imports)]
pub use project::{ProjectDetail, ProjectStatus, ProjectSummary};
#[allow(unused_imports)]
pub use task::{
    TaskDetail, TaskDraft, TaskEdit, TaskFinishParams, TaskNoteParams, TaskPriority,
    TaskStartParams, TaskStatus, TaskSummary,
};
pub use user::{display_mapping, filter_users, match_users, UserMatch, UserSummary};

/// 分页结果。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub total_pages: u64,
}

impl<T> Page<T> {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            page: 1,
            per_page: 20,
            total_pages: 0,
        }
    }
}

/// 通用 ID 包装，避免到处传递原始字符串/整数。
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct EntityId(pub String);

impl From<&str> for EntityId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for EntityId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
