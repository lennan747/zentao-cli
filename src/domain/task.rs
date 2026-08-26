use serde::{Deserialize, Serialize};

use crate::domain::EntityId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Wait,
    Doing,
    Done,
    Paused,
    Cancel,
    Closed,
    #[default]
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskStatus::Wait => "wait",
            TaskStatus::Doing => "doing",
            TaskStatus::Done => "done",
            TaskStatus::Paused => "paused",
            TaskStatus::Cancel => "cancel",
            TaskStatus::Closed => "closed",
            TaskStatus::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    #[serde(rename = "1")]
    Low,
    #[serde(rename = "2")]
    Normal,
    #[serde(rename = "3")]
    High,
    #[serde(rename = "4")]
    Urgent,
    #[default]
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskPriority::Low => "1",
            TaskPriority::Normal => "2",
            TaskPriority::High => "3",
            TaskPriority::Urgent => "4",
            TaskPriority::Unknown => "?",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: EntityId,
    pub project_id: EntityId,
    pub project_name: String,
    pub name: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    #[serde(default)]
    pub assigned_to: String,
    #[serde(default)]
    pub deadline: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDetail {
    pub id: EntityId,
    pub project_id: EntityId,
    pub project_name: String,
    pub name: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    #[serde(default)]
    pub assigned_to: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub opened_by: String,
    #[serde(default)]
    pub opened_date: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub estimate: f64,
    #[serde(default)]
    pub consumed: f64,
    #[serde(default)]
    pub left: f64,
}

/// 新建任务草稿。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskDraft {
    pub name: String,
    pub desc: Option<String>,
    pub module: Option<String>,
    pub task_type: Option<String>,
    pub pri: Option<String>,
    pub estimate: Option<String>,
    pub est_started: Option<String>,
    pub deadline: Option<String>,
    pub assigned_to: Option<String>,
    pub mailto: Vec<String>,
}

/// 编辑任务变更集；仅提交用户指定的字段。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskEdit {
    pub name: Option<String>,
    pub desc: Option<String>,
    pub assigned_to: Option<String>,
    pub pri: Option<String>,
    pub task_type: Option<String>,
    pub status: Option<String>,
    pub estimate: Option<String>,
    pub consumed: Option<String>,
    pub left: Option<String>,
    pub deadline: Option<String>,
    pub est_started: Option<String>,
    pub comment: Option<String>,
}

/// 开始任务参数。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskStartParams {
    pub real_started: Option<String>,
    pub consumed: Option<String>,
    pub left: Option<String>,
    pub assigned_to: Option<String>,
    pub comment: Option<String>,
}

/// 完成任务参数。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskFinishParams {
    pub current_consumed: Option<String>,
    pub left: Option<String>,
    pub finished_date: Option<String>,
    pub assigned_to: Option<String>,
    pub comment: Option<String>,
}

/// 取消/关闭/激活任务的共用参数（仅备注）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskNoteParams {
    pub comment: Option<String>,
}
