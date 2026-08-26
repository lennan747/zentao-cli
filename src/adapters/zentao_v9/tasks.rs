use async_trait::async_trait;
use serde_json::Value;

use crate::application::{TaskGateway, TaskQuery};
use crate::domain::{
    EntityId, Page, QueryError, TaskDetail, TaskDraft, TaskEdit, TaskFinishParams, TaskNoteParams,
    TaskPriority, TaskStartParams, TaskStatus, TaskSummary,
};

use super::client::ZentaoV9Client;
use super::normalize::{enum_field, num_field, opt_date, str_field};
use super::response::{parse_alert_response, parse_body};
use super::routes::Routes;

/// 组装表单字段：仅保留有值的字段。
pub(super) fn optional_fields(fields: Vec<(&str, Option<String>)>) -> Vec<(String, String)> {
    fields
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k.to_string(), v)))
        .collect()
}

/// 组装必填字段。
pub(super) fn field(k: &str, v: impl Into<String>) -> (String, String) {
    (k.to_string(), v.into())
}

/// 从对象 JSON 复制表单字段（json_key -> form_name）。缺失字段跳过；
/// `0000-00-00` 归零日期视为空串（与页面表单行为一致）。
pub(super) fn form_from_json(
    json: &Value,
    fields: &[(&str, &str)],
) -> Vec<(String, String)> {
    fields
        .iter()
        .filter_map(|(json_key, form_name)| {
            let value = json.get(*json_key)?;
            let text = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => value.as_str().unwrap_or("").to_string(),
            };
            let text = if let Some(rest) = text.strip_prefix("0000-00-00") {
                rest.trim().to_string()
            } else {
                text
            };
            Some((form_name.to_string(), text))
        })
        .collect()
}

/// 把用户提供的可选字段覆盖到表单中；`Some` 覆盖，`None` 不动。
pub(super) fn override_fields(
    form: &mut Vec<(String, String)>,
    overrides: Vec<(&str, Option<String>)>,
) {
    for (name, value) in overrides {
        let Some(value) = value else { continue };
        match form.iter_mut().find(|(k, _)| k == name) {
            Some(slot) => slot.1 = value,
            None => form.push((name.to_string(), value)),
        }
    }
}

/// 把逗号分隔的账号列表转成重复表单字段（mailto 等）。
pub(super) fn split_accounts(form: &mut Vec<(String, String)>, name: &str, raw: &str) {
    for account in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        form.push((name.to_string(), account.to_string()));
    }
}

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

    /// 提交写表单并解析包络；成功返回 Ok。
    async fn post_write(&self, url: &str, form: Vec<(String, String)>) -> Result<(), QueryError> {
        let pairs: Vec<(&str, &str)> = form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let body = self.client.post_form_text(url, &pairs).await?;
        parse_body(&body).map(|_| ())
    }

    /// 读取当前任务的原始 JSON（编辑表单的全量基线）。
    async fn raw_task(&self, id: &EntityId) -> Result<Value, QueryError> {
        let body = self
            .client
            .get_text(&Routes::task_view(self.client.server(), &id.0))
            .await?;
        let data = parse_body(&body)?;
        data.get("task")
            .cloned()
            .ok_or(QueryError::IncompatibleResponse)
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

    async fn create_task(&self, project: EntityId, draft: TaskDraft) -> Result<(), QueryError> {
        let mut form = vec![field("name", draft.name)];
        for (k, v) in optional_fields(vec![
            ("desc", draft.desc),
            ("pri", draft.pri),
            ("type", draft.task_type),
            ("estimate", draft.estimate),
            ("estStarted", draft.est_started),
            ("deadline", draft.deadline),
            ("module", draft.module),
            ("assignedTo[]", draft.assigned_to),
        ]) {
            form.push((k, v));
        }
        for account in draft.mailto {
            form.push(field("mailto[]", account));
        }
        let url = Routes::task_create(self.client.server(), &project.0);
        self.post_write(&url, form).await
    }

    async fn edit_task(&self, id: EntityId, edit: TaskEdit) -> Result<(), QueryError> {
        // 旧版编辑语义：**未提交的表单字段会被服务端清空**（2026-08-26 真实环境确认）。
        // 因此必须先把当前对象的完整字段作为基线，再覆盖用户指定的变更。
        //
        // 例外：`status` 不随基线提交——实例存在工作流规则（如 doing+剩余=0 时状态必须为 done），
        // 提交当前状态会触发无关校验；仅当用户明确要求 `--status` 时才提交。
        let raw = self.raw_task(&id).await?;
        let mut form = form_from_json(
            &raw,
            &[
                ("name", "name"),
                ("desc", "desc"),
                ("estimate", "estimate"),
                ("left", "left"),
                ("deadline", "deadline"),
                ("estStarted", "estStarted"),
                ("realStarted", "realStarted"),
                ("finishedDate", "finishedDate"),
                ("canceledDate", "canceledDate"),
                ("closedDate", "closedDate"),
                ("consumed", "consumed"),
                ("assignedTo", "assignedTo"),
                ("pri", "pri"),
                ("type", "type"),
                ("module", "module"),
                ("story", "story"),
                ("project", "project"),
            ],
        );
        if let Some(mailto) = raw.get("mailto").and_then(|v| v.as_str()) {
            split_accounts(&mut form, "mailto[]", mailto);
        }
        // 多人任务团队基线：成员与工时数组必须一并提交，否则成员会被清空。
        if let Some(team) = raw.get("team").and_then(|v| v.as_object()) {
            let mut rows: Vec<_> = team.iter().collect();
            rows.sort_by_key(|(_, m)| {
                m.get("order")
                    .and_then(|o| o.as_str())
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            });
            if !rows.is_empty() {
                form.push(field("multiple", "1"));
                for (account, member) in rows {
                    form.push(field("team[]", account));
                    let num = |k: &str| -> String {
                        member
                            .get(k)
                            .map(|v| v.as_str().unwrap_or_default().to_string())
                            .unwrap_or_default()
                    };
                    form.push(("teamEstimate[]".to_string(), num("estimate")));
                    form.push(("teamConsumed[]".to_string(), num("consumed")));
                    form.push(("teamLeft[]".to_string(), num("left")));
                }
            }
        }
        override_fields(
            &mut form,
            vec![
                ("name", edit.name),
                ("desc", edit.desc),
                ("assignedTo", edit.assigned_to),
                ("pri", edit.pri),
                ("type", edit.task_type),
                ("status", edit.status),
                ("estimate", edit.estimate),
                ("consumed", edit.consumed),
                ("left", edit.left),
                ("deadline", edit.deadline),
                ("estStarted", edit.est_started),
                ("comment", edit.comment),
            ],
        );
        let url = Routes::task_edit(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn start_task(&self, id: EntityId, params: TaskStartParams) -> Result<(), QueryError> {
        let mut form = vec![field("status", "doing")];
        form.extend(optional_fields(vec![
            ("realStarted", params.real_started),
            ("consumed", params.consumed),
            ("left", params.left),
            ("assignedTo", params.assigned_to),
            ("comment", params.comment),
        ]));
        let url = Routes::task_start(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn finish_task(&self, id: EntityId, params: TaskFinishParams) -> Result<(), QueryError> {
        let mut form = vec![field("status", "done")];
        form.extend(optional_fields(vec![
            ("currentConsumed", params.current_consumed),
            ("left", params.left),
            ("finishedDate", params.finished_date),
            ("assignedTo", params.assigned_to),
            ("comment", params.comment),
        ]));
        let url = Routes::task_finish(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn cancel_task(&self, id: EntityId, params: TaskNoteParams) -> Result<(), QueryError> {
        let mut form = vec![field("status", "cancel")];
        form.extend(optional_fields(vec![("comment", params.comment)]));
        let url = Routes::task_cancel(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn close_task(&self, id: EntityId, params: TaskNoteParams) -> Result<(), QueryError> {
        let mut form = vec![field("status", "closed")];
        form.extend(optional_fields(vec![("comment", params.comment)]));
        let url = Routes::task_close(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn activate_task(&self, id: EntityId, params: TaskNoteParams) -> Result<(), QueryError> {
        let mut form = vec![field("status", "wait")];
        form.extend(optional_fields(vec![("comment", params.comment)]));
        let url = Routes::task_activate(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn comment_task(&self, id: EntityId, comment: &str) -> Result<(), QueryError> {
        let form = [("comment".to_string(), comment.to_string())];
        let pairs: Vec<(&str, &str)> = form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let url = Routes::task_comment(self.client.server(), &id.0);
        let body = self.client.post_form_text(&url, &pairs).await?;
        parse_alert_response(&body)
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
