use comfy_table::Table;
use serde::Serialize;

use crate::cli::commands::OutputFormat;
use crate::domain::{BugSummary, Page, ProjectSummary, TaskSummary};

/// 将单个可序列化值按指定格式输出到 stdout。
pub fn print_value<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(value)?;
            println!("{json}");
        }
        OutputFormat::Table => {
            // 单个对象也序列化为 JSON，table 格式在业务子任务中再细化。
            let json = serde_json::to_string_pretty(value)?;
            println!("{json}");
        }
    }
    Ok(())
}

/// 将分页项目列表按指定格式输出。
pub fn print_project_page(
    page: &Page<ProjectSummary>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(page)?);
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.set_header(["ID", "Code", "Name", "Status"]);
            for item in &page.items {
                table.add_row([
                    item.id.to_string(),
                    item.code.clone(),
                    item.name.clone(),
                    item.status.to_string(),
                ]);
            }
            println!("{table}");
            println!("Total: {}, Page: {}/{}", page.total, page.page, page.total_pages);
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
            println!("Total: {}, Page: {}/{}", page.total, page.page, page.total_pages);
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
            table.set_header(["ID", "Project", "Title", "Status", "Severity", "Priority", "Assigned To"]);
            for item in &page.items {
                table.add_row([
                    item.id.to_string(),
                    item.project_id.to_string(),
                    item.title.clone(),
                    item.status.to_string(),
                    item.severity.to_string(),
                    item.priority.to_string(),
                    item.assigned_to.clone(),
                ]);
            }
            println!("{table}");
            println!("Total: {}, Page: {}/{}", page.total, page.page, page.total_pages);
        }
    }
    Ok(())
}
