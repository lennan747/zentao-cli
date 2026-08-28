use std::process::ExitCode;

use clap::{Args, Subcommand};

use super::{fail, load_session_client, ok, CommandContext};
use crate::adapters::zentao_v9::ZentaoV9TaskGateway;
use crate::application::{TaskGateway, TaskQuery};
use crate::cli::confirm::{confirm_write, WriteControl, WriteFlags};
use crate::cli::output;
use crate::domain::{
    EntityId, TaskDraft, TaskEdit, TaskFinishParams, TaskNoteParams, TaskStartParams, TaskStatus,
    ZentaoError,
};

#[derive(Debug, Clone, Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TaskCommands {
    /// 列出我的任务
    List(ListArgs),
    /// 查看任务详情
    Get(GetArgs),
    /// 创建任务
    Create(CreateArgs),
    /// 编辑任务
    Edit(EditArgs),
    /// 指派任务（通过编辑接口提交指派人）
    Assign(AssignArgs),
    /// 开始任务（wait -> doing）
    Start(StartArgs),
    /// 完成任务（doing -> done）
    Finish(FinishArgs),
    /// 取消任务
    Cancel(CancelArgs),
    /// 关闭任务
    Close(CloseArgs),
    /// 激活任务（done/cancel/closed -> wait）
    Activate(ActivateArgs),
    /// 评论任务
    Comment(CommentArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    /// 指派对象（旧版接口仅支持 me）
    #[arg(short, long)]
    pub assigned_to: Option<String>,
    /// 状态过滤（wait/doing/done/pause/cancel/closed），本地过滤
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct GetArgs {
    /// 任务 ID
    pub id: String,
}

#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
    /// 项目 ID（任务必须归属某项目）
    pub project: String,

    /// 任务名称
    #[arg(long)]
    pub name: String,

    /// 任务描述
    #[arg(long)]
    pub desc: Option<String>,

    /// 优先级 1-4
    #[arg(long)]
    pub pri: Option<String>,

    /// 任务类型（design/devel/test/study/discuss/ui/affair/misc/production/management）
    #[arg(long, value_name = "TYPE")]
    pub r#type: Option<String>,

    /// 预计工时（小时）
    #[arg(long)]
    pub estimate: Option<String>,

    /// 预计开始日期 YYYY-MM-DD
    #[arg(long)]
    pub est_started: Option<String>,

    /// 截止日期 YYYY-MM-DD
    #[arg(long)]
    pub deadline: Option<String>,

    /// 所属模块 ID（0=根）
    #[arg(long)]
    pub module: Option<String>,

    /// 指派给（账号）
    #[arg(long)]
    pub assigned_to: Option<String>,

    /// 抄送账号（可多次）
    #[arg(long)]
    pub mailto: Vec<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct EditArgs {
    /// 任务 ID
    pub id: String,

    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub desc: Option<String>,
    /// 指派人账号
    #[arg(long)]
    pub assigned_to: Option<String>,
    /// 优先级 0-4
    #[arg(long)]
    pub pri: Option<String>,
    /// 任务类型
    #[arg(long, value_name = "TYPE")]
    pub r#type: Option<String>,
    /// 状态（wait/doing/done/pause/cancel/closed）
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub estimate: Option<String>,
    #[arg(long)]
    pub consumed: Option<String>,
    #[arg(long)]
    pub left: Option<String>,
    #[arg(long)]
    pub deadline: Option<String>,
    #[arg(long)]
    pub est_started: Option<String>,
    /// 备注/评论
    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct AssignArgs {
    /// 任务 ID
    pub id: String,
    /// 指派给（账号）
    pub account: String,
    /// 备注/评论
    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct StartArgs {
    /// 任务 ID
    pub id: String,

    /// 实际开始时间 YYYY-MM-DD HH:MM:SS（缺省用服务端当前时间）
    #[arg(long)]
    pub real_started: Option<String>,
    /// 本次消耗工时（小时，缺省 0）
    #[arg(long)]
    pub consumed: Option<String>,
    /// 剩余工时（小时）。必须大于 0：为 0 时禅道会把“开始”当作“完成”
    #[arg(long)]
    pub left: Option<String>,
    /// 开始后指派给（账号，缺省不变）
    #[arg(long)]
    pub assigned_to: Option<String>,
    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct FinishArgs {
    /// 任务 ID
    pub id: String,

    /// 本次消耗工时（小时，必须大于 0）
    #[arg(long, required = true)]
    pub consumed: String,
    /// 完成日期 YYYY-MM-DD（缺省用服务端当前时间）
    #[arg(long)]
    pub finished_date: Option<String>,
    /// 完成后指派给（账号，缺省不变）
    #[arg(long)]
    pub assigned_to: Option<String>,
    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct CancelArgs {
    /// 任务 ID
    pub id: String,

    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct CloseArgs {
    /// 任务 ID
    pub id: String,

    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct ActivateArgs {
    /// 任务 ID
    pub id: String,

    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct CommentArgs {
    /// 任务 ID
    pub id: String,

    /// 评论内容
    #[arg(long)]
    pub comment: String,

    #[command(flatten)]
    pub write: WriteFlags,
}

/// 生成操作摘要。
fn summary(title: &str, items: &[(&str, &str)]) -> String {
    crate::cli::output::render_summary(title, items)
}

macro_rules! try_write {
    ($flags:expr, $summary:expr, $call:expr) => {{
        match confirm_write(&$summary, $flags) {
            Ok(WriteControl::Aborted) => return ok(),
            Ok(WriteControl::Proceed) => {}
            Err(e) => return fail(&e),
        }
        match $call.await {
            Ok(()) => {
                println!("{}", crate::cli::style::green("已提交成功"));
                ok()
            }
            Err(e) => fail(&e.into()),
        }
    }};
}

pub async fn handle(args: TaskArgs, ctx: &CommandContext) -> ExitCode {
    let client = match load_session_client(&ctx.profile) {
        Ok(c) => c,
        Err(e) => return fail(&e),
    };
    let gateway = ZentaoV9TaskGateway::new(client);

    match args.command {
        TaskCommands::List(list) => {
            match gateway
                .list_tasks(TaskQuery {
                    assigned_to: list.assigned_to.filter(|s| !s.trim().is_empty()),
                    status: list.status.filter(|s| !s.trim().is_empty()),
                    ..Default::default()
                })
                .await
            {
                Ok(page) => output::print_task_page(&page, ctx.format)
                    .map_err(|e| ZentaoError::Internal(e.to_string()))
                    .map_or_else(|e| fail(&e), |_| ok()),
                Err(e) => fail(&e.into()),
            }
        }
        TaskCommands::Get(get) => match gateway.get_task(EntityId::from(get.id)).await {
            Ok(detail) => output::print_value(&detail, ctx.format)
                .map_err(|e| ZentaoError::Internal(e.to_string()))
                .map_or_else(|e| fail(&e), |_| ok()),
            Err(e) => fail(&e.into()),
        },
        TaskCommands::Create(a) => {
            for item in [("name", &a.name)] {
                if item.1.trim().is_empty() {
                    return fail(&ZentaoError::Query(
                        crate::domain::QueryError::InvalidParameter("任务名称不能为空".into()),
                    ));
                }
            }
            let draft = TaskDraft {
                name: a.name.clone(),
                desc: a.desc.clone(),
                module: a.module.clone(),
                task_type: a.r#type.clone(),
                pri: a.pri.clone(),
                estimate: a.estimate.clone(),
                est_started: a.est_started.clone(),
                deadline: a.deadline.clone(),
                assigned_to: a.assigned_to.clone(),
                mailto: a.mailto.clone(),
            };
            let s = summary(
                &format!("创建任务（项目 {}）", a.project),
                &[
                    ("name", draft.name.as_str()),
                    ("desc", draft.desc.as_deref().unwrap_or("")),
                    ("pri", draft.pri.as_deref().unwrap_or("")),
                    ("type", draft.task_type.as_deref().unwrap_or("")),
                    ("estimate", draft.estimate.as_deref().unwrap_or("")),
                    ("estStarted", draft.est_started.as_deref().unwrap_or("")),
                    ("deadline", draft.deadline.as_deref().unwrap_or("")),
                    ("module", draft.module.as_deref().unwrap_or("")),
                    ("assignedTo", draft.assigned_to.as_deref().unwrap_or("")),
                ],
            );
            try_write!(
                a.write,
                s,
                gateway.create_task(EntityId::from(a.project.as_str()), draft.clone())
            )
        }
        TaskCommands::Edit(a) => {
            // 编辑至少要提供一个字段，避免无意义提交。
            let edit = TaskEdit {
                name: a.name.clone(),
                desc: a.desc.clone(),
                assigned_to: a.assigned_to.clone(),
                pri: a.pri.clone(),
                task_type: a.r#type.clone(),
                status: a.status.clone(),
                estimate: a.estimate.clone(),
                consumed: a.consumed.clone(),
                left: a.left.clone(),
                deadline: a.deadline.clone(),
                est_started: a.est_started.clone(),
                comment: a.comment.clone(),
            };
            if all_none(&edit) {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter("未提供任何要修改的字段".into()),
                ));
            }
            let s = summary(
                &format!("编辑任务 {}", a.id),
                &[
                    ("name", edit.name.as_deref().unwrap_or("")),
                    ("desc", edit.desc.as_deref().unwrap_or("")),
                    ("assignedTo", edit.assigned_to.as_deref().unwrap_or("")),
                    ("pri", edit.pri.as_deref().unwrap_or("")),
                    ("type", edit.task_type.as_deref().unwrap_or("")),
                    ("status", edit.status.as_deref().unwrap_or("")),
                    ("estimate", edit.estimate.as_deref().unwrap_or("")),
                    ("consumed", edit.consumed.as_deref().unwrap_or("")),
                    ("left", edit.left.as_deref().unwrap_or("")),
                    ("deadline", edit.deadline.as_deref().unwrap_or("")),
                    ("estStarted", edit.est_started.as_deref().unwrap_or("")),
                    ("comment", edit.comment.as_deref().unwrap_or("")),
                ],
            );
            let s =
                format!("{s}  备注：将连当前字段基线一并提交（旧版接口行为，空提交会清空字段）");
            try_write!(
                a.write,
                s,
                gateway.edit_task(EntityId::from(a.id.as_str()), edit.clone())
            )
        }
        TaskCommands::Assign(a) => {
            let edit = TaskEdit {
                assigned_to: Some(a.account.clone()),
                comment: a.comment.clone(),
                ..Default::default()
            };
            let s = summary(
                &format!("指派任务 {}", a.id),
                &[
                    ("assignedTo", a.account.as_str()),
                    ("comment", a.comment.as_deref().unwrap_or("")),
                ],
            );
            try_write!(
                a.write,
                s,
                gateway.edit_task(EntityId::from(a.id.as_str()), edit.clone())
            )
        }
        TaskCommands::Start(a) => {
            let id = EntityId::from(a.id.as_str());
            let detail = match gateway.get_task(id.clone()).await {
                Ok(d) => d,
                Err(e) => return fail(&e.into()),
            };
            let status = detail.status.to_string();
            if !matches!(detail.status, TaskStatus::Wait | TaskStatus::Paused) {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter(format!(
                        "只能开始 wait/pause 状态的任务（当前状态: {status}）"
                    )),
                ));
            }
            let left = match a.left.as_deref().filter(|s| !s.trim().is_empty()) {
                Some(v) => v.trim().to_string(),
                None => format!("{}", detail.left),
            };
            if left.parse::<f64>().map(|v| v == 0.0).unwrap_or(false) {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter(
                        "left（剩余工时）为 0：禅道会把“开始”当作“完成”并指派回创建人。\
                     请用 --left 指定剩余工时；任务已无剩余时请直接用 task finish"
                            .into(),
                    ),
                ));
            }
            let consumed = a
                .consumed
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("0")
                .to_string();
            let p = TaskStartParams {
                real_started: a.real_started.clone(),
                consumed: Some(consumed.clone()),
                left: Some(left.clone()),
                assigned_to: a.assigned_to.clone(),
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("开始任务 {}（当前状态: {status}）", a.id),
                &[
                    (
                        "realStarted",
                        p.real_started
                            .as_deref()
                            .unwrap_or("（缺省：服务端当前时间）"),
                    ),
                    ("consumed", consumed.as_str()),
                    ("left", left.as_str()),
                    ("assignedTo", p.assigned_to.as_deref().unwrap_or("（不变）")),
                    ("comment", p.comment.as_deref().unwrap_or("")),
                ],
            );
            try_write!(a.write, s, gateway.start_task(id, p.clone()))
        }
        TaskCommands::Finish(a) => {
            let id = EntityId::from(a.id.as_str());
            let detail = match gateway.get_task(id.clone()).await {
                Ok(d) => d,
                Err(e) => return fail(&e.into()),
            };
            let status = detail.status.to_string();
            if !matches!(detail.status, TaskStatus::Wait | TaskStatus::Doing) {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter(format!(
                        "只能完成 wait/doing 状态的任务（当前状态: {status}）"
                    )),
                ));
            }
            let consumed = a.consumed.trim();
            match consumed.parse::<f64>() {
                Ok(v) if v > 0.0 => {}
                _ => {
                    return fail(&ZentaoError::Query(
                        crate::domain::QueryError::InvalidParameter(
                            "本次消耗（--consumed）必须大于 0".into(),
                        ),
                    ));
                }
            }
            let p = TaskFinishParams {
                current_consumed: Some(consumed.to_string()),
                finished_date: a.finished_date.clone(),
                assigned_to: a.assigned_to.clone(),
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!(
                    "完成任务 {}（当前状态: {status}，此前总计消耗: {}）",
                    a.id, detail.consumed
                ),
                &[
                    ("consumed（基线）", &format!("{}", detail.consumed)),
                    ("currentConsumed", consumed),
                    (
                        "finishedDate",
                        p.finished_date
                            .as_deref()
                            .unwrap_or("（缺省：服务端当前时间）"),
                    ),
                    (
                        "assignedTo",
                        p.assigned_to
                            .as_deref()
                            .unwrap_or(detail.assigned_to.as_str()),
                    ),
                    ("comment", p.comment.as_deref().unwrap_or("")),
                ],
            );
            try_write!(a.write, s, gateway.finish_task(id, p.clone()))
        }
        TaskCommands::Cancel(a) => {
            let id = EntityId::from(a.id.as_str());
            let detail = match gateway.get_task(id.clone()).await {
                Ok(d) => d,
                Err(e) => return fail(&e.into()),
            };
            let status = detail.status.to_string();
            if !matches!(
                detail.status,
                TaskStatus::Wait | TaskStatus::Doing | TaskStatus::Paused
            ) {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter(format!(
                        "不能取消 {status} 状态的任务（仅 wait/doing/pause 可取消）"
                    )),
                ));
            }
            let p = TaskNoteParams {
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("取消任务 {}（当前状态: {status}）", a.id),
                &[("comment", p.comment.as_deref().unwrap_or(""))],
            );
            try_write!(a.write, s, gateway.cancel_task(id, p.clone()))
        }
        TaskCommands::Close(a) => {
            let id = EntityId::from(a.id.as_str());
            let detail = match gateway.get_task(id.clone()).await {
                Ok(d) => d,
                Err(e) => return fail(&e.into()),
            };
            let status = detail.status.to_string();
            if detail.status != TaskStatus::Done {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter(format!(
                        "只能关闭 done 状态的任务（当前状态: {status}）"
                    )),
                ));
            }
            let p = TaskNoteParams {
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("关闭任务 {}（当前状态: {status}）", a.id),
                &[("comment", p.comment.as_deref().unwrap_or(""))],
            );
            try_write!(a.write, s, gateway.close_task(id, p.clone()))
        }
        TaskCommands::Activate(a) => {
            let id = EntityId::from(a.id.as_str());
            let detail = match gateway.get_task(id.clone()).await {
                Ok(d) => d,
                Err(e) => return fail(&e.into()),
            };
            let status = detail.status.to_string();
            if !matches!(
                detail.status,
                TaskStatus::Done | TaskStatus::Cancel | TaskStatus::Closed
            ) {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter(format!(
                        "只能激活 done/cancel/closed 状态的任务（当前状态: {status}）"
                    )),
                ));
            }
            let p = TaskNoteParams {
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("激活任务 {}（当前状态: {status}）", a.id),
                &[("comment", p.comment.as_deref().unwrap_or(""))],
            );
            try_write!(a.write, s, gateway.activate_task(id, p.clone()))
        }
        TaskCommands::Comment(a) => {
            if a.comment.trim().is_empty() {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter("评论内容不能为空".into()),
                ));
            }
            let s = summary(
                &format!("评论任务 {}", a.id),
                &[("comment", a.comment.as_str())],
            );
            try_write!(
                a.write,
                s,
                gateway.comment_task(EntityId::from(a.id.as_str()), &a.comment)
            )
        }
    }
}

fn all_none(edit: &TaskEdit) -> bool {
    edit.name.is_none()
        && edit.desc.is_none()
        && edit.assigned_to.is_none()
        && edit.pri.is_none()
        && edit.task_type.is_none()
        && edit.status.is_none()
        && edit.estimate.is_none()
        && edit.consumed.is_none()
        && edit.left.is_none()
        && edit.deadline.is_none()
        && edit.est_started.is_none()
        && edit.comment.is_none()
}
