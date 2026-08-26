use serde::{Deserialize, Serialize};

use crate::domain::EntityId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BugStatus {
    Active,
    Resolved,
    Closed,
    #[default]
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for BugStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BugStatus::Active => "active",
            BugStatus::Resolved => "resolved",
            BugStatus::Closed => "closed",
            BugStatus::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BugSeverity {
    #[serde(rename = "1")]
    One,
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "3")]
    Three,
    #[serde(rename = "4")]
    Four,
    #[default]
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for BugSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BugSeverity::One => "1",
            BugSeverity::Two => "2",
            BugSeverity::Three => "3",
            BugSeverity::Four => "4",
            BugSeverity::Unknown => "?",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BugSummary {
    pub id: EntityId,
    pub product_id: EntityId,
    pub project_id: EntityId,
    pub title: String,
    pub status: BugStatus,
    pub severity: BugSeverity,
    pub priority: u8,
    #[serde(default)]
    pub assigned_to: String,
    #[serde(default)]
    pub opened_by: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BugDetail {
    pub id: EntityId,
    pub product_id: EntityId,
    pub product_name: String,
    pub project_id: EntityId,
    pub title: String,
    pub status: BugStatus,
    pub severity: BugSeverity,
    pub priority: u8,
    #[serde(default)]
    pub assigned_to: String,
    #[serde(default)]
    pub opened_by: String,
    #[serde(default)]
    pub opened_date: Option<String>,
    #[serde(default)]
    pub steps: String,
}

/// 新建 Bug 草稿。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BugDraft {
    pub title: String,
    pub steps: Option<String>,
    pub module: Option<String>,
    pub project: Option<String>,
    pub severity: Option<String>,
    pub pri: Option<String>,
    pub assigned_to: Option<String>,
    pub opened_build: Option<String>,
    pub deadline: Option<String>,
    pub keywords: Option<String>,
    pub bug_type: Option<String>,
    pub os: Option<String>,
    pub browser: Option<String>,
    pub mailto: Vec<String>,
}

/// 编辑 Bug 变更集；仅提交用户指定的字段。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BugEdit {
    pub title: Option<String>,
    pub steps: Option<String>,
    pub severity: Option<String>,
    pub pri: Option<String>,
    pub assigned_to: Option<String>,
    pub status: Option<String>,
    pub resolution: Option<String>,
    pub resolved_build: Option<String>,
    pub opened_build: Option<String>,
    pub deadline: Option<String>,
    pub keywords: Option<String>,
    pub bug_type: Option<String>,
    pub os: Option<String>,
    pub browser: Option<String>,
    pub comment: Option<String>,
}

/// 解决 Bug 参数。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BugResolveParams {
    pub resolution: Option<String>,
    pub resolved_build: Option<String>,
    pub build_name: Option<String>,
    pub assigned_to: Option<String>,
    pub comment: Option<String>,
}

/// 激活 Bug 参数。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BugActivateParams {
    pub assigned_to: Option<String>,
    pub opened_build: Option<String>,
    pub comment: Option<String>,
}

/// 关闭/确认 Bug 的共用参数（仅备注）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BugNoteParams {
    pub comment: Option<String>,
}
