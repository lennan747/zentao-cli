use comfy_table::Table;
use serde::Serialize;
use serde_json::Value;

use crate::cli::commands::OutputFormat;
use crate::domain::{BugSummary, Page, ProjectSummary, TaskSummary};

/// 将单个可序列化值按指定格式输出到 stdout。
///
/// table 格式输出字段/值两列表；json 格式输出合法 JSON（FORMAT-001）。
pub fn print_value<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Table => {
            let value = serde_json::to_value(value)?;
            if let Value::Object(map) = value {
                let mut table = Table::new();
                table.set_header(["Field", "Value"]);
                for (key, val) in map {
                    table.add_row([key, value_cell(&val)]);
                }
                println!("{table}");
            } else {
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        }
    }
    Ok(())
}

fn value_cell(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// 将分页项目列表按指定格式输出。
///
/// `/project-index.json` 只提供 id→名称映射，完整字段见 `project get`。
pub fn print_project_page(page: &Page<ProjectSummary>, format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(page)?);
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.set_header(["ID", "Name"]);
            for item in &page.items {
                table.add_row([item.id.to_string(), item.name.clone()]);
            }
            println!("{table}");
            println!("Total: {}", page.total);
        }
    }
    Ok(())
}

/// 将分页任务列表按指定格式输出。
pub fn print_task_page(page: &Page<TaskSummary>, format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(page)?);
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.set_header(["ID", "Project", "Name", "Status", "Priority", "Assigned To"]);
            for item in &page.items {
                table.add_row([
                    item.id.to_string(),
                    item.project_name.clone(),
                    item.name.clone(),
                    item.status.to_string(),
                    item.priority.to_string(),
                    item.assigned_to.clone(),
                ]);
            }
            println!("{table}");
            println!(
                "Total: {}, Page: {}/{}",
                page.total, page.page, page.total_pages
            );
        }
    }
    Ok(())
}

/// 将分页 Bug 列表按指定格式输出。
pub fn print_bug_page(page: &Page<BugSummary>, format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(page)?);
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.set_header([
                "ID",
                "Product",
                "Title",
                "Status",
                "Severity",
                "Priority",
                "Assigned To",
            ]);
            for item in &page.items {
                table.add_row([
                    item.id.to_string(),
                    item.product_id.to_string(),
                    item.title.clone(),
                    item.status.to_string(),
                    item.severity.to_string(),
                    item.priority.to_string(),
                    item.assigned_to.clone(),
                ]);
            }
            println!("{table}");
            println!(
                "Total: {}, Page: {}/{}",
                page.total, page.page, page.total_pages
            );
        }
    }
    Ok(())
}
