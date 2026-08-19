use async_trait::async_trait;
use serde_json::Value;

use crate::application::{TaskGateway, TaskQuery};
use crate::domain::{
    EntityId, Page, QueryError, TaskDetail, TaskPriority, TaskStatus, TaskSummary,
};

use super::client::ZentaoV9Client;
use super::normalize::{enum_field, num_field, opt_date, str_field};
use super::response::parse_body;
use super::routes::Routes;

/// 任务查询网关（禅道 V9 旧版 `.json` 接口）。
///
/// 列表数据来自 `/my-task.json`（“我的任务”），服务端分页参数尚未确认，
/// `--status` / `--assigned-to` 过滤在本地完成。
pub struct ZentaoV9TaskGateway {
    client: ZentaoV9Client,
}

impl ZentaoV9TaskGateway {
    pub fn new(client: ZentaoV9Client) -> Self {
        Self { client }
    }

    fn summary_from(value: &Value) -> Option<TaskSummary> {
        Some(TaskSummary {
            id: EntityId::from(str_field(value, "id")),
            project_id: EntityId::from(str_field(value, "project")),
            project_name: str_field(value, "projectName"),
            name: str_field(value, "name"),
            status: enum_field::<TaskStatus>(value, "status"),
            priority: enum_field::<TaskPriority>(value, "pri"),
            assigned_to: str_field(value, "assignedTo"),
            deadline: opt_date(value, "deadline"),
        })
    }

    fn detail_from(data: &Value) -> Result<TaskDetail, QueryError> {
        let task = data.get("task").ok_or(QueryError::IncompatibleResponse)?;
        let project_name = data
            .get("project")
            .map(|p| str_field(p, "name"))
            .unwrap_or_else(|| str_field(task, "projectName"));
        Ok(TaskDetail {
            id: EntityId::from(str_field(task, "id")),
            project_id: EntityId::from(str_field(task, "project")),
            project_name,
            name: str_field(task, "name"),
            status: enum_field::<TaskStatus>(task, "status"),
            priority: enum_field::<TaskPriority>(task, "pri"),
            assigned_to: str_field(task, "assignedTo"),
            desc: super::normalize::strip_html(&str_field(task, "desc")),
            opened_by: str_field(task, "openedBy"),
            opened_date: opt_date(task, "openedDate"),
            deadline: opt_date(task, "deadline"),
            estimate: num_field(task, "estimate"),
            consumed: num_field(task, "consumed"),
            left: num_field(task, "left"),
        })
    }
}

#[async_trait]
impl TaskGateway for ZentaoV9TaskGateway {
    async fn list_tasks(&self, query: TaskQuery) -> Result<Page<TaskSummary>, QueryError> {
        if let Some(assigned) = query.assigned_to.as_deref() {
            if assigned != "me" {
                return Err(QueryError::InvalidParameter(
                    "旧版接口仅支持 --assigned-to me（我的任务）".into(),
                ));
            }
        }

        let body = self
            .client
            .get_text(&Routes::my_task(self.client.server()))
            .await?;
        let data = parse_body(&body)?;

        let tasks = data
            .get("tasks")
            .and_then(|v| v.as_array())
            .ok_or(QueryError::IncompatibleResponse)?;

        let mut items: Vec<TaskSummary> = tasks.iter().filter_map(Self::summary_from).collect();
        let fallback_total = items.len() as u64;

        if let Some(status) = query.status.as_deref() {
            if !status.is_empty() {
                items.retain(|t| t.status.to_string() == status);
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

    async fn get_task(&self, id: EntityId) -> Result<TaskDetail, QueryError> {
        let body = self
            .client
            .get_text(&Routes::task_view(self.client.server(), &id.0))
            .await?;
        let data = parse_body(&body)?;
        Self::detail_from(&data)
    }
}

/// 读取 pager 对象；缺失时退化为列表长度。返回 (page, per_page, total_pages, total)。
pub(super) fn pager_info(data: &Value, fallback_total: u64) -> (u64, u64, u64, u64) {
    let pager = data.get("pager");
    let total = pager
        .and_then(|p| p.get("recTotal"))
        .and_then(|v| v.as_u64())
        .unwrap_or(fallback_total);
    let page = pager
        .and_then(|p| p.get("pageID"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1);
    let per_page = pager
        .and_then(|p| p.get("recPerPage"))
        .and_then(|v| v.as_u64())
        .unwrap_or(20);
    let total_pages = pager
        .and_then(|p| p.get("pageTotal"))
        .and_then(|v| v.as_u64())
        .unwrap_or(if total == 0 { 0 } else { 1 });
    (page, per_page, total_pages, total)
}
