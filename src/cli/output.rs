use std::io::IsTerminal;

use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Row, Table};
use html_escape::decode_html_entities;
use serde::Serialize;
use serde_json::Value;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::cli::commands::OutputFormat;
use crate::cli::style;
use crate::domain::{
    BugSeverity, BugStatus, BugSummary, Page, ProjectSummary, TaskPriority, TaskStatus, TaskSummary,
};

/// Field/Value 表格中 Value 列的最大显示宽度（30 个全角字符）。
const MAX_VALUE_WIDTH: usize = 60;

/// 列表表格中 Name/Title 列的最大显示宽度（约 20 个全角字符），超长截断为 `...`。
const NAME_MAX_WIDTH: u16 = 40;

/// 按显示宽度把长文本折成多行，超过 `max_width` 才换行。
///
/// 保留原文中的换行；单个字符为全角（中文等）时按 2 列计宽。
fn wrap_value(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());

    for (line_idx, line) in text.split('\n').enumerate() {
        if line_idx > 0 {
            out.push('\n');
        }
        let mut line_width = 0usize;
        for ch in line.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if line_width > 0 && line_width + w > max_width {
                out.push('\n');
                line_width = 0;
            }
            out.push(ch);
            line_width += w;
        }
    }
    out
}

/// 把服务端文本中的 HTML 实体（`&amp;` 等）解码为字面字符。
fn clean_display(text: &str) -> String {
    decode_html_entities(text).into_owned()
}

/// 按显示宽度截断文本，超出 `max_width` 的追加 `...`。
fn truncate_display(text: &str, max_width: usize) -> String {
    let cleaned = clean_display(text);
    if UnicodeWidthStr::width(cleaned.as_str()) <= max_width {
        return cleaned;
    }

    let limit = max_width.saturating_sub(3);
    let mut out = String::new();
    let mut width = 0usize;
    for ch in cleaned.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + w > limit {
            break;
        }
        out.push(ch);
        width += w;
    }
    out.push_str("...");
    out
}

fn new_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_HORIZONTAL_ONLY);
    table
}

/// 终端下开启自适应列宽（按终端宽度动态分布），非 TTY 回退内容自适应。
fn adapt_width(table: &mut Table) {
    if !std::io::stdout().is_terminal() {
        return;
    }
    if let Ok((width, _)) = crossterm::terminal::size() {
        if width > 0 {
            table
                .set_width(width)
                .set_content_arrangement(ContentArrangement::Dynamic);
        }
    }
}

fn header_row(labels: &[&str]) -> Row {
    Row::from(
        labels
            .iter()
            .map(|label| {
                Cell::new(*label)
                    .add_attribute(Attribute::Bold)
                    .fg(Color::White)
                    .bg(Color::DarkGrey)
            })
            .collect::<Vec<Cell>>(),
    )
}

fn plain(text: impl ToString) -> Cell {
    Cell::new(text)
}

fn colored_opt(text: impl ToString, color: Option<Color>) -> Cell {
    match color {
        Some(color) => Cell::new(text).fg(color),
        None => plain(text),
    }
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Wait => "○ 等待中",
        TaskStatus::Doing => "● 进行中",
        TaskStatus::Done => "✔ 已完成",
        TaskStatus::Paused => "⏸ 已暂停",
        TaskStatus::Cancel => "✘ 已取消",
        TaskStatus::Closed => "已关闭",
        TaskStatus::Unknown => "-",
    }
}

fn task_status_cell(status: TaskStatus) -> Cell {
    let color = match status {
        TaskStatus::Wait | TaskStatus::Paused => Color::Yellow,
        TaskStatus::Doing => Color::Cyan,
        TaskStatus::Done => Color::Green,
        TaskStatus::Cancel => Color::Red,
        TaskStatus::Closed | TaskStatus::Unknown => Color::DarkGrey,
    };
    Cell::new(task_status_label(status)).fg(color)
}

fn bug_status_label(status: BugStatus) -> &'static str {
    match status {
        BugStatus::Active => "● 处理中",
        BugStatus::Resolved => "✔ 已解决",
        BugStatus::Closed => "已关闭",
        BugStatus::Unknown => "-",
    }
}

fn bug_status_cell(status: BugStatus) -> Cell {
    let color = match status {
        BugStatus::Active => Color::Yellow,
        BugStatus::Resolved => Color::Green,
        BugStatus::Closed | BugStatus::Unknown => Color::DarkGrey,
    };
    Cell::new(bug_status_label(status)).fg(color)
}

fn task_priority_cell(priority: TaskPriority) -> Cell {
    let text = priority.to_string();
    match priority {
        TaskPriority::Low | TaskPriority::Normal => Cell::new(text)
            .fg(Color::Red)
            .add_attribute(Attribute::Bold),
        TaskPriority::High | TaskPriority::Urgent => Cell::new(text).fg(Color::Green),
        TaskPriority::Unknown => Cell::new(text),
    }
}

fn bug_priority_cell(priority: u8) -> Cell {
    let text = priority.to_string();
    match priority {
        1 | 2 => Cell::new(text)
            .fg(Color::Red)
            .add_attribute(Attribute::Bold),
        3 | 4 => Cell::new(text).fg(Color::Green),
        _ => Cell::new(text),
    }
}

fn severity_cell(severity: BugSeverity) -> Cell {
    let color = match severity {
        BugSeverity::One => Some(Color::Red),
        BugSeverity::Two => Some(Color::DarkYellow),
        BugSeverity::Three => Some(Color::Yellow),
        BugSeverity::Four | BugSeverity::Unknown => None,
    };
    colored_opt(severity.to_string(), color)
}

/// 把内部字段键名映射为中文标签；未知字段回退原键名。
fn field_label(key: &str) -> &str {
    match key {
        "id" => "ID",
        "name" => "名称",
        "title" => "标题",
        "desc" => "描述",
        "status" => "状态",
        "priority" | "pri" => "优先级",
        "severity" => "严重程度",
        "type" => "类型",
        "estimate" => "预计工时",
        "consumed" => "消耗工时",
        "left" => "剩余工时",
        "deadline" => "截止日期",
        "module" => "模块",
        "comment" => "备注",
        "steps" => "重现步骤",
        "keywords" => "关键词",
        "os" => "操作系统",
        "browser" => "浏览器",
        "project" => "所属项目",
        "project_id" => "所属项目",
        "project_name" => "项目",
        "product_id" => "所属产品",
        "product_name" => "产品",
        "opened_by" => "创建者",
        "opened_date" => "创建日期",
        "code" => "编号",
        "pm" => "项目负责人",
        "begin" => "开始日期",
        "end" => "结束日期",
        "assignedTo" | "assigned_to" => "指派给",
        "realStarted" => "实际开始时间",
        "estStarted" => "预计开始",
        "currentConsumed" => "本次消耗",
        "finishedDate" => "完成日期",
        "openedBuild" => "版本",
        "resolution" => "解决方案",
        "resolvedBuild" => "解决版本",
        "buildName" => "版本名称",
        "resolvedDate" => "解决日期",
        "resolvedBy" => "解决者",
        "closedDate" => "关闭日期",
        "closedBy" => "关闭者",
        "confirmed" => "确认",
        "assignedDate" => "指派日期",
        "activatedDate" => "激活日期",
        "consumed（基线）" => "消耗工时（基线）",
        "history" => "历史记录",
        _ => key,
    }
}

/// 把详情中 status 字段的枚举值映射为中文标签；无法识别时返回 `None`。
fn status_value_label(value: &str) -> Option<&'static str> {
    let label = match value {
        "wait" => "○ 等待中",
        "doing" => "● 进行中",
        "done" => "✔ 已完成",
        "paused" | "suspended" => "⏸ 已暂停",
        "cancel" => "✘ 已取消",
        "closed" => "已关闭",
        "active" => "● 处理中",
        "resolved" => "✔ 已解决",
        _ => return None,
    };
    Some(label)
}

fn print_empty() {
    println!("{}", style::dim("（无数据）"));
}

/// 将写操作确认摘要渲染为对齐文本块（无尾随换行）。
pub fn render_summary(title: &str, items: &[(&str, &str)]) -> String {
    let max_key = items
        .iter()
        .map(|(key, _)| UnicodeWidthStr::width(field_label(key)))
        .max()
        .unwrap_or(0);
    let mut lines = vec![style::bold(title)];
    for (key, value) in items {
        let label = field_label(key);
        let rendered = if value.is_empty() {
            style::dim("-")
        } else if value.starts_with('（') {
            style::dim(value)
        } else {
            (*value).to_string()
        };
        let pad = max_key - UnicodeWidthStr::width(label);
        lines.push(format!("  {label}{}  {rendered}", " ".repeat(pad)));
    }
    lines.join("\n")
}

/// 将单个可序列化值按指定格式输出到 stdout。
///
/// table 格式输出字段/值两列表；json 格式输出合法 JSON（FORMAT-001）。
/// `table_fields` 为 `Some` 时按给定键/标签清单逐行渲染（仅显示清单字段，
/// 标签可覆盖 `field_label` 默认映射）；为 `None` 时按 `key_display_order`
/// 排序渲染全部字段。
pub fn print_value<T: Serialize>(
    value: &T,
    format: OutputFormat,
    table_fields: Option<&[(&str, &str)]>,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Table => {
            let value = serde_json::to_value(value)?;
            if let Value::Object(map) = value {
                let mut table = new_table();
                table.set_header(header_row(&["字段", "值"]));
                let ordered: Vec<(String, Option<String>)> = match table_fields {
                    Some(fields) => fields
                        .iter()
                        .map(|(k, l)| (k.to_string(), Some(l.to_string())))
                        .collect(),
                    None => {
                        let mut keys: Vec<String> = map.keys().cloned().collect();
                        keys.sort_by_key(|k| key_display_order(k));
                        keys.into_iter().map(|k| (k, None)).collect()
                    }
                };
                for (key, label_override) in ordered {
                    if key.ends_with("_images") {
                        // 图片 URL 并入对应富文本字段（steps/desc）单元格，不单独成行
                        continue;
                    }
                    let Some(val) = map.get(&key) else {
                        continue;
                    };
                    let raw = clean_display(&value_cell(val));
                    let mut cell_value = match key.as_str() {
                        "status" => status_value_label(&raw).unwrap_or(&raw).to_string(),
                        "history" => history_cell(val),
                        _ => raw,
                    };
                    // 富文本内嵌图片：URL 追加为该单元格的独立行（与禅道页面一致）
                    let images_key = format!("{key}_images");
                    if let Some(Value::Array(items)) = map.get(&images_key) {
                        let urls: Vec<&str> = items.iter().filter_map(|x| x.as_str()).collect();
                        if !urls.is_empty() {
                            for u in urls {
                                cell_value.push('\n');
                                cell_value.push_str(u);
                            }
                        }
                    }
                    let label = label_override
                        .as_deref()
                        .unwrap_or_else(|| field_label(&key));
                    table.add_row([
                        plain(label),
                        plain(wrap_value(&cell_value, MAX_VALUE_WIDTH)),
                    ]);
                }
                println!("{table}");
            } else {
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        }
    }
    Ok(())
}

/// Bug 详情表格的字段清单、顺序与标签（白名单，隐藏 指派给/优先级/所属产品/严重程度）。
pub const BUG_DETAIL_TABLE_FIELDS: &[(&str, &str)] = &[
    ("id", "ID"),
    ("status", "状态"),
    ("product_name", "产品"),
    ("project_name", "所属项目"),
    ("title", "标题"),
    ("steps", "重现步骤"),
    ("opened_by", "创建者"),
    ("opened_date", "创建日期"),
    ("history", "历史记录"),
];

/// 详情表格默认字段顺序（`table_fields` 为 None 时使用）。
///
/// 返回序号越小越靠前；未列出的键排最后（保持稳定排序，即字母序）。
fn key_display_order(key: &str) -> usize {
    const ORDER: &[&str] = &[
        "id",
        "name",
        "code",
        "title",
        "status",
        "product_id",
        "product_name",
        "project_id",
        "project_name",
        "severity",
        "priority",
        "pri",
        "assigned_to",
        "estimate",
        "consumed",
        "left",
        "deadline",
        "desc",
        "steps",
        "opened_by",
        "opened_date",
        "pm",
        "begin",
        "end",
        "history",
    ];
    ORDER
        .iter()
        .position(|k| *k == key)
        .map(|i| i + 1)
        .unwrap_or(usize::MAX)
}

/// 把动作英文名映射为中文标签；未知动作原样返回。
fn action_label(action: &str) -> &str {
    match action {
        "opened" => "创建",
        "assigned" => "指派给",
        "edited" => "编辑",
        "commented" => "备注",
        "resolved" => "解决",
        "closed" => "关闭",
        "activated" => "激活",
        "confirmed" => "确认",
        "reopened" => "重新激活",
        "started" => "开始",
        "finished" => "完成",
        "canceled" | "cancelled" => "取消",
        "deleted" => "删除",
        "moved" => "移动",
        "restarted" => "重启",
        _ => action,
    }
}

/// 把历史记录 JSON 数组渲染为多行文本，每个动作/字段变更一行。
fn history_cell(value: &Value) -> String {
    let Value::Array(entries) = value else {
        return value_cell(value);
    };
    let mut lines: Vec<String> = Vec::new();
    for entry in entries {
        let date = entry.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let actor = entry.get("actor").and_then(|v| v.as_str()).unwrap_or("");
        let action = entry.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let comment = entry.get("comment").and_then(|v| v.as_str()).unwrap_or("");

        let mut prefix = String::new();
        if !date.is_empty() {
            prefix.push_str(date);
            prefix.push(' ');
        }
        if !actor.is_empty() {
            prefix.push_str(actor);
            prefix.push(' ');
        }
        prefix.push_str(action_label(action));

        let changes = entry.get("fields").and_then(|v| v.as_array());
        if action == "commented" {
            // 备注动作：正文即备注文本
            let mut line = prefix;
            if !comment.is_empty() {
                line.push_str(": ");
                line.push_str(&clean_display(comment));
            }
            lines.push(line);
            continue;
        }
        match changes {
            Some(items) if !items.is_empty() => {
                for item in items {
                    let field = item.get("field").and_then(|v| v.as_str()).unwrap_or("");
                    let old = item.get("old").and_then(|v| v.as_str()).unwrap_or("");
                    let new = item.get("new").and_then(|v| v.as_str()).unwrap_or("");
                    let mut line = prefix.clone();
                    line.push_str(": ");
                    line.push_str(field_label(field));
                    if old.is_empty() {
                        line.push_str(" = ");
                        line.push_str(&clean_display(new));
                    } else if new.is_empty() {
                        line.push_str(" 清除 ");
                        line.push_str(&clean_display(old));
                    } else {
                        line.push(' ');
                        line.push_str(&clean_display(old));
                        line.push_str(" → ");
                        line.push_str(&clean_display(new));
                    }
                    lines.push(line);
                }
                if is_note(comment) {
                    lines.push(format!("{prefix} 备注: {}", clean_display(comment)));
                }
            }
            _ => {
                if is_note(comment) {
                    lines.push(format!("{prefix} 备注: {}", clean_display(comment)));
                } else {
                    lines.push(prefix);
                }
            }
        }
    }
    lines.join("\n")
}

/// 备注文本是否有效（非空且不是纯数字——旧版把备注编号也放在 `comment` 字段里）。
fn is_note(text: &str) -> bool {
    !text.trim().is_empty() && !text.trim().chars().all(|c| c.is_ascii_digit())
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
            if page.items.is_empty() {
                print_empty();
                return Ok(());
            }
            let mut table = new_table();
            table.set_header(header_row(&["ID", "名称"]));
            adapt_width(&mut table);
            for item in &page.items {
                table.add_row([
                    plain(item.id.to_string()),
                    plain(truncate_display(&item.name, NAME_MAX_WIDTH as usize)),
                ]);
            }
            println!("{table}");
            println!("{}", style::dim(&format!("Total: {}", page.total)));
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
            if page.items.is_empty() {
                print_empty();
                return Ok(());
            }
            let mut table = new_table();
            table.set_header(header_row(&[
                "ID",
                "项目",
                "名称",
                "状态",
                "优先级",
                "指派给",
            ]));
            adapt_width(&mut table);
            for item in &page.items {
                table.add_row([
                    plain(item.id.to_string()),
                    plain(truncate_display(
                        &item.project_name,
                        NAME_MAX_WIDTH as usize,
                    )),
                    plain(truncate_display(&item.name, NAME_MAX_WIDTH as usize)),
                    task_status_cell(item.status),
                    task_priority_cell(item.priority),
                    plain(clean_display(&item.assigned_to)),
                ]);
            }
            println!("{table}");
            println!(
                "{}",
                style::dim(&format!(
                    "Total: {}, Page: {}/{}",
                    page.total, page.page, page.total_pages
                ))
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
            if page.items.is_empty() {
                print_empty();
                return Ok(());
            }
            let mut table = new_table();
            table.set_header(header_row(&[
                "ID",
                "产品",
                "标题",
                "状态",
                "严重程度",
                "优先级",
                "指派给",
            ]));
            adapt_width(&mut table);
            for item in &page.items {
                table.add_row([
                    plain(item.id.to_string()),
                    plain(item.product_id.to_string()),
                    plain(truncate_display(&item.title, NAME_MAX_WIDTH as usize)),
                    bug_status_cell(item.status),
                    severity_cell(item.severity),
                    bug_priority_cell(item.priority),
                    plain(clean_display(&item.assigned_to)),
                ]);
            }
            println!("{table}");
            println!(
                "{}",
                style::dim(&format!(
                    "Total: {}, Page: {}/{}",
                    page.total, page.page, page.total_pages
                ))
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        clean_display, field_label, render_summary, status_value_label, task_status_label,
        truncate_display, wrap_value,
    };
    use crate::domain::TaskStatus;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn wrap_value_wraps_cjk_at_display_width() {
        let s = "中".repeat(40);
        let wrapped = wrap_value(&s, 60);
        let lines: Vec<&str> = wrapped.split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].chars().count(), 30);
        assert_eq!(lines[1].chars().count(), 10);
    }

    #[test]
    fn wrap_value_wraps_ascii_at_display_width() {
        let s = "a".repeat(80);
        let wrapped = wrap_value(&s, 60);
        let lines: Vec<&str> = wrapped.split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), 60);
        assert_eq!(lines[1].len(), 20);
    }

    #[test]
    fn wrap_value_preserves_existing_newlines_and_short_text() {
        let wrapped = wrap_value("第一行\n第二行", 60);
        assert_eq!(wrapped, "第一行\n第二行");

        let short = "短文本".repeat(3);
        assert_eq!(wrap_value(&short, 60), short);
    }

    #[test]
    fn clean_display_decodes_html_entities() {
        assert_eq!(clean_display("A &amp; B"), "A & B");
        assert_eq!(clean_display("1 &lt; 2 &gt; 0"), "1 < 2 > 0");
        assert_eq!(clean_display("他说&quot;好&quot;"), "他说\"好\"");
        assert_eq!(clean_display("无实体文本"), "无实体文本");
    }

    #[test]
    fn truncate_display_truncates_long_cjk_with_ellipsis() {
        let long = "超长任务名称".repeat(20);
        let truncated = truncate_display(&long, 40);
        assert!(truncated.ends_with("..."));
        assert!(UnicodeWidthStr::width(truncated.as_str()) <= 40);
        assert_ne!(truncated, long);
    }

    #[test]
    fn truncate_display_keeps_short_text() {
        assert_eq!(truncate_display("短名", 40), "短名");
        assert_eq!(truncate_display("A &amp; B", 40), "A & B");
    }

    #[test]
    fn status_labels_use_chinese_symbols() {
        assert_eq!(task_status_label(TaskStatus::Doing), "● 进行中");
        assert_eq!(task_status_label(TaskStatus::Wait), "○ 等待中");
        assert_eq!(task_status_label(TaskStatus::Done), "✔ 已完成");
        assert_eq!(task_status_label(TaskStatus::Cancel), "✘ 已取消");
    }

    #[test]
    fn field_label_maps_known_keys_and_falls_back() {
        assert_eq!(field_label("assignedTo"), "指派给");
        assert_eq!(field_label("assigned_to"), "指派给");
        assert_eq!(field_label("project_name"), "项目");
        assert_eq!(field_label("consumed（基线）"), "消耗工时（基线）");
        assert_eq!(field_label("realStarted"), "实际开始时间");
        assert_eq!(field_label("unknown_key"), "unknown_key");
    }

    #[test]
    fn status_value_label_maps_status_values() {
        assert_eq!(status_value_label("doing"), Some("● 进行中"));
        assert_eq!(status_value_label("active"), Some("● 处理中"));
        assert_eq!(status_value_label("resolved"), Some("✔ 已解决"));
        assert_eq!(status_value_label("suspended"), Some("⏸ 已暂停"));
        assert_eq!(status_value_label("bogus"), None);
    }

    #[test]
    fn render_summary_aligns_keys_and_uses_dash_for_empty() {
        let s = render_summary(
            "开始任务 979（当前状态: wait）",
            &[
                ("realStarted", "（缺省：服务端当前时间）"),
                ("consumed", "0"),
                ("left", "1"),
                ("assignedTo", "（不变）"),
                ("comment", ""),
            ],
        );

        assert!(s.starts_with("开始任务 979（当前状态: wait）\n"));
        assert!(s.contains("  实际开始时间  （缺省：服务端当前时间）"));
        assert!(s.contains("  消耗工时      0"));
        assert!(s.contains("  剩余工时      1"));
        assert!(s.contains("  指派给        （不变）"));
        assert!(s.ends_with("  备注          -"));
        assert!(!s.ends_with('\n'));
        assert!(!s.contains('='));
    }
}
