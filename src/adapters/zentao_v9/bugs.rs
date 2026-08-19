use async_trait::async_trait;
use serde_json::Value;

use crate::application::{BugGateway, BugQuery};
use crate::domain::{BugDetail, BugSeverity, BugStatus, BugSummary, EntityId, Page, QueryError};

use super::client::ZentaoV9Client;
use super::normalize::{enum_field, num_field, opt_date, str_field};
use super::response::parse_body;
use super::routes::Routes;
use super::tasks::pager_info;

/// Bug 查询网关（禅道 V9 旧版 `.json` 接口）。
///
/// 列表数据来自 `/my-bug-assignedTo.json`（“指派给我”），`--status` 过滤在本地完成。
pub struct ZentaoV9BugGateway {
    client: ZentaoV9Client,
}

impl ZentaoV9BugGateway {
    pub fn new(client: ZentaoV9Client) -> Self {
        Self { client }
    }

    fn summary_from(value: &Value) -> Option<BugSummary> {
        Some(BugSummary {
            id: EntityId::from(str_field(value, "id")),
            product_id: EntityId::from(str_field(value, "product")),
            project_id: EntityId::from(str_field(value, "project")),
            title: str_field(value, "title"),
            status: enum_field::<BugStatus>(value, "status"),
            severity: enum_field::<BugSeverity>(value, "severity"),
            priority: num_field(value, "pri"),
            assigned_to: str_field(value, "assignedTo"),
            opened_by: str_field(value, "openedBy"),
        })
    }

    fn detail_from(data: &Value) -> Result<BugDetail, QueryError> {
        let bug = data.get("bug").ok_or(QueryError::IncompatibleResponse)?;
        Ok(BugDetail {
            id: EntityId::from(str_field(bug, "id")),
            product_id: EntityId::from(str_field(bug, "product")),
            product_name: str_field(data, "productName"),
            project_id: EntityId::from(str_field(bug, "project")),
            title: str_field(bug, "title"),
            status: enum_field::<BugStatus>(bug, "status"),
            severity: enum_field::<BugSeverity>(bug, "severity"),
            priority: num_field(bug, "pri"),
            assigned_to: str_field(bug, "assignedTo"),
            opened_by: str_field(bug, "openedBy"),
            opened_date: opt_date(bug, "openedDate"),
            steps: super::normalize::strip_html(&str_field(bug, "steps")),
        })
    }
}

#[async_trait]
impl BugGateway for ZentaoV9BugGateway {
    async fn list_bugs(&self, query: BugQuery) -> Result<Page<BugSummary>, QueryError> {
        if let Some(assigned) = query.assigned_to.as_deref() {
            if assigned != "me" {
                return Err(QueryError::InvalidParameter(
                    "旧版接口仅支持 --assigned-to me（指派给我）".into(),
                ));
            }
        }

        let body = self
            .client
            .get_text(&Routes::my_bug_assigned_to(self.client.server()))
            .await?;
        let data = parse_body(&body)?;

        let bugs = data
            .get("bugs")
            .and_then(|v| v.as_array())
            .ok_or(QueryError::IncompatibleResponse)?;

        let mut items: Vec<BugSummary> = bugs.iter().filter_map(Self::summary_from).collect();
        let fallback_total = items.len() as u64;

        if let Some(status) = query.status.as_deref() {
            if !status.is_empty() {
                items.retain(|b| b.status.to_string() == status);
            }
        }

        let (page_id, per_page, total_pages, total) = pager_info(&data, fallback_total);
        Ok(Page {
            items,
            total,
            page: page_id,
            per_page,
            total_pages,
        })
    }

    async fn get_bug(&self, id: EntityId) -> Result<BugDetail, QueryError> {
        let body = self
            .client
            .get_text(&Routes::bug_view(self.client.server(), &id.0))
            .await?;
        let data = parse_body(&body)?;
        Self::detail_from(&data)
    }
}
