use async_trait::async_trait;

use crate::domain::{EntityId, Page, ProjectDetail, ProjectSummary, QueryError};

#[derive(Debug, Clone, Default)]
pub struct ProjectQuery {
    pub status: Option<String>,
    pub page: u64,
    pub per_page: u64,
}

/// 项目查询端口。
#[async_trait]
pub trait ProjectGateway: Send + Sync {
    async fn list_projects(&self, query: ProjectQuery) -> Result<Page<ProjectSummary>, QueryError>;
    async fn get_project(&self, id: EntityId) -> Result<ProjectDetail, QueryError>;
}
