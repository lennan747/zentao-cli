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
