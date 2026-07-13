use serde::{Deserialize, Serialize};

use crate::domain::EntityId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Wait,
    Doing,
    Done,
    Suspended,
    Closed,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ProjectStatus::Wait => "wait",
            ProjectStatus::Doing => "doing",
            ProjectStatus::Done => "done",
            ProjectStatus::Suspended => "suspended",
            ProjectStatus::Closed => "closed",
            ProjectStatus::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: EntityId,
    pub code: String,
    pub name: String,
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub id: EntityId,
    pub code: String,
    pub name: String,
    pub status: ProjectStatus,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub pm: String,
    #[serde(default)]
    pub begin: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}
