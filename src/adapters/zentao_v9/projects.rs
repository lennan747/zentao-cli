use async_trait::async_trait;
use serde_json::Value;

use crate::application::{ProjectGateway, ProjectQuery};
use crate::domain::{EntityId, Page, ProjectDetail, ProjectStatus, ProjectSummary, QueryError};

use super::client::ZentaoV9Client;
use super::normalize::{enum_field, opt_date, str_field};
use super::response::parse_body;
use super::routes::Routes;

/// 项目查询网关（禅道 V9 旧版 `.json` 接口）。
pub struct ZentaoV9ProjectGateway {
    client: ZentaoV9Client,
}

impl ZentaoV9ProjectGateway {
    pub fn new(client: ZentaoV9Client) -> Self {
        Self { client }
    }

    fn detail_from(value: &Value) -> Result<ProjectDetail, QueryError> {
        let project = value
            .get("project")
            .ok_or(QueryError::IncompatibleResponse)?;
        Ok(ProjectDetail {
            id: EntityId::from(str_field(project, "id")),
            code: str_field(project, "code"),
            name: str_field(project, "name"),
            status: enum_field::<ProjectStatus>(project, "status"),
            desc: super::normalize::strip_html(&str_field(project, "desc")),
            pm: str_field(project, "PM"),
            begin: opt_date(project, "begin"),
            end: opt_date(project, "end"),
        })
    }
}

#[async_trait]
impl ProjectGateway for ZentaoV9ProjectGateway {
    async fn list_projects(&self, query: ProjectQuery) -> Result<Page<ProjectSummary>, QueryError> {
        let server = self.client.server();
        let url = match query.status.as_deref() {
            // 端点矩阵确认：status=0 表示全部，与 project-index 等价。
            Some(status) if !status.is_empty() && status != "all" && status != "0" => {
                Routes::project_all(server, status)
            }
            _ => Routes::project_index(server),
        };

        let body = self.client.get_text(&url).await?;
        let data = parse_body(&body)?;
        let projects = data
            .get("projects")
            .and_then(|v| v.as_object())
            .ok_or(QueryError::IncompatibleResponse)?;

        let mut items: Vec<ProjectSummary> = projects
            .iter()
            .filter(|(id, _)| id.parse::<u64>().is_ok())
            .map(|(id, name)| ProjectSummary {
                id: EntityId::from(id.as_str()),
                name: name.as_str().unwrap_or_default().to_string(),
            })
            .collect();
        items.sort_by_key(|p| p.id.0.parse::<u64>().unwrap_or(u64::MAX));

        let total = items.len() as u64;
        Ok(Page {
            per_page: total.max(1),
            items,
            total,
            page: 1,
            total_pages: 1,
        })
    }

    async fn get_project(&self, id: EntityId) -> Result<ProjectDetail, QueryError> {
        let body = self
            .client
            .get_text(&Routes::project_view(self.client.server(), &id.0))
            .await?;
        let data = parse_body(&body)?;
        Self::detail_from(&data)
    }
}
