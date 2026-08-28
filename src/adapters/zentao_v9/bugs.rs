use async_trait::async_trait;
use serde_json::Value;

use crate::application::{BugGateway, BugQuery};
use crate::domain::{
    BugActivateParams, BugDetail, BugDraft, BugEdit, BugNoteParams, BugResolveParams, BugSeverity,
    BugStatus, BugSummary, EntityId, Page, QueryError,
};

use super::client::ZentaoV9Client;
use super::normalize::{enum_field, num_field, opt_date, str_field};
use super::response::{parse_alert_response, parse_body};
use super::routes::Routes;
use super::tasks::{
    field, form_from_json, optional_fields, override_fields, pager_info, split_accounts,
};

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

    /// 提交写表单并解析包络；成功返回 Ok。
    async fn post_write(&self, url: &str, form: Vec<(String, String)>) -> Result<(), QueryError> {
        let pairs: Vec<(&str, &str)> = form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let body = self.client.post_form_text(url, &pairs).await?;
        parse_body(&body).map(|_| ())
    }

    /// 读取当前 Bug 的影响版本（编辑/激活表单的必填基线）。
    async fn raw_bug(&self, id: &EntityId) -> Result<Value, QueryError> {
        let body = self
            .client
            .get_text(&Routes::bug_view(self.client.server(), &id.0))
            .await?;
        let data = parse_body(&body)?;
        data.get("bug")
            .cloned()
            .ok_or(QueryError::IncompatibleResponse)
    }

    /// 读取当前 Bug 的影响版本（编辑/激活表单的必填基线）。
    async fn current_opened_build(&self, id: &EntityId) -> Result<Option<String>, QueryError> {
        let bug = self.raw_bug(id).await?;
        Ok(match bug.get("openedBuild") {
            Some(Value::Array(items)) => {
                let parts: Vec<String> = items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .filter(|s| !s.is_empty() && s != "0")
                    .collect();
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join(","))
                }
            }
            Some(Value::String(s)) if !s.is_empty() && s != "0" => Some(s.clone()),
            _ => None,
        })
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

    fn detail_from(data: &Value, server: &str) -> Result<BugDetail, QueryError> {
        let bug = data.get("bug").ok_or(QueryError::IncompatibleResponse)?;
        let raw_steps = str_field(bug, "steps");
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
            steps_images: super::normalize::resolve_image_urls(&raw_steps, server),
            steps: super::normalize::strip_html(&raw_steps),
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
        Self::detail_from(&data, self.client.server())
    }

    async fn create_bug(&self, product: EntityId, draft: BugDraft) -> Result<(), QueryError> {
        let mut form = vec![field("title", draft.title)];
        form.extend(optional_fields(vec![
            ("steps", draft.steps),
            ("severity", draft.severity),
            ("pri", draft.pri),
            ("type", draft.bug_type),
            ("os", draft.os),
            ("browser", draft.browser),
            ("deadline", draft.deadline),
            ("keywords", draft.keywords),
            ("module", draft.module),
            ("project", draft.project),
            ("assignedTo", draft.assigned_to),
            ("openedBuild[]", draft.opened_build),
        ]));
        for account in draft.mailto {
            form.push(field("mailto[]", account));
        }
        let url = Routes::bug_create(self.client.server(), &product.0);
        self.post_write(&url, form).await
    }

    async fn edit_bug(&self, id: EntityId, edit: BugEdit) -> Result<(), QueryError> {
        // 旧版编辑语义：**未提交的表单字段会被服务端清空**（2026-08-26 真实环境确认）。
        // 因此必须先把当前对象的完整字段作为基线，再覆盖用户指定的变更。
        let raw = self.raw_bug(&id).await?;
        // 与任务编辑同理：`status` 不随基线提交，避免触发实例工作流校验。
        let mut form = form_from_json(
            &raw,
            &[
                ("title", "title"),
                ("keywords", "keywords"),
                ("severity", "severity"),
                ("pri", "pri"),
                ("type", "type"),
                ("os", "os"),
                ("browser", "browser"),
                ("deadline", "deadline"),
                ("steps", "steps"),
                ("product", "product"),
                ("module", "module"),
                ("plan", "plan"),
                ("story", "story"),
                ("task", "task"),
                ("project", "project"),
                ("assignedTo", "assignedTo"),
                ("resolvedBuild", "resolvedBuild"),
                ("resolution", "resolution"),
            ],
        );
        if let Some(build) = raw.get("openedBuild") {
            let list: Vec<String> = match build {
                Value::String(s) => s
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect(),
                Value::Array(items) => items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => vec![],
            };
            for b in list {
                form.push(field("openedBuild[]", b));
            }
        }
        if let Some(mailto) = raw.get("mailto").and_then(|v| v.as_str()) {
            split_accounts(&mut form, "mailto[]", mailto);
        }
        override_fields(
            &mut form,
            vec![
                ("title", edit.title),
                ("steps", edit.steps),
                ("severity", edit.severity),
                ("pri", edit.pri),
                ("assignedTo", edit.assigned_to),
                ("status", edit.status),
                ("resolution", edit.resolution),
                ("resolvedBuild", edit.resolved_build),
                ("deadline", edit.deadline),
                ("keywords", edit.keywords),
                ("type", edit.bug_type),
                ("os", edit.os),
                ("browser", edit.browser),
                ("comment", edit.comment),
            ],
        );
        if let Some(build) = edit.opened_build {
            let mut found = false;
            for (k, v) in form.iter_mut() {
                if k == "openedBuild[]" {
                    *v = build.clone();
                    found = true;
                    break;
                }
            }
            if !found {
                form.push(("openedBuild[]".to_string(), build));
            }
        }
        let url = Routes::bug_edit(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn resolve_bug(&self, id: EntityId, params: BugResolveParams) -> Result<(), QueryError> {
        let mut form = vec![field("status", "resolved")];
        form.extend(optional_fields(vec![
            ("resolution", params.resolution),
            ("resolvedBuild", params.resolved_build),
            ("buildName", params.build_name),
            ("assignedTo", params.assigned_to),
            ("comment", params.comment),
        ]));
        let url = Routes::bug_resolve(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn activate_bug(
        &self,
        id: EntityId,
        params: BugActivateParams,
    ) -> Result<(), QueryError> {
        let opened_build = match params.opened_build.as_deref() {
            Some(v) => v.to_string(),
            None => self.current_opened_build(&id).await?.unwrap_or_default(),
        };
        let mut form = vec![field("status", "active")];
        if !opened_build.is_empty() {
            form.push(field("openedBuild[]", opened_build));
        }
        form.extend(optional_fields(vec![
            ("assignedTo", params.assigned_to),
            ("comment", params.comment),
        ]));
        let url = Routes::bug_activate(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn close_bug(&self, id: EntityId, params: BugNoteParams) -> Result<(), QueryError> {
        let mut form = vec![field("status", "closed")];
        form.extend(optional_fields(vec![("comment", params.comment)]));
        let url = Routes::bug_close(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn confirm_bug(&self, id: EntityId, params: BugNoteParams) -> Result<(), QueryError> {
        let form = optional_fields(vec![("comment", params.comment)]);
        let url = Routes::bug_confirm(self.client.server(), &id.0);
        self.post_write(&url, form).await
    }

    async fn comment_bug(&self, id: EntityId, comment: &str) -> Result<(), QueryError> {
        let form = [("comment".to_string(), comment.to_string())];
        let pairs: Vec<(&str, &str)> = form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let url = Routes::bug_comment(self.client.server(), &id.0);
        let body = self.client.post_form_text(&url, &pairs).await?;
        parse_alert_response(&body)
    }
}
