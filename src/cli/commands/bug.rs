use std::process::ExitCode;

use clap::{Args, Subcommand};

use super::{fail, load_session_client, ok, CommandContext};
use crate::adapters::zentao_v9::ZentaoV9BugGateway;
use crate::application::{BugGateway, BugQuery};
use crate::cli::confirm::{confirm_write, WriteControl, WriteFlags};
use crate::cli::output;
use crate::domain::{
    BugActivateParams, BugDraft, BugEdit, BugNoteParams, BugResolveParams, EntityId, ZentaoError,
};

#[derive(Debug, Clone, Args)]
pub struct BugArgs {
    #[command(subcommand)]
    pub command: BugCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum BugCommands {
    /// 列出指派给我的 Bug
    List(ListArgs),
    /// 查看 Bug 详情
    Get(GetArgs),
    /// 创建 Bug
    Create(CreateArgs),
    /// 编辑 Bug
    Edit(EditArgs),
    /// 指派 Bug（通过编辑接口提交指派人）
    Assign(AssignArgs),
    /// 解决 Bug（-> resolved）
    Resolve(ResolveArgs),
    /// 激活 Bug（resolved/closed -> active）
    Activate(ActivateArgs),
    /// 关闭 Bug
    Close(CloseArgs),
    /// 确认 Bug
    Confirm(ConfirmArgs),
    /// 评论 Bug
    Comment(CommentArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    /// 指派对象（旧版接口仅支持 me）
    #[arg(short, long)]
    pub assigned_to: Option<String>,
    /// 状态过滤（active/resolved/closed），本地过滤
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct GetArgs {
    /// Bug ID
    pub id: String,
}

#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
    /// 产品 ID（Bug 必须归属某产品）
    pub product: String,

    /// Bug 标题
    #[arg(long)]
    pub title: String,

    /// 复现步骤
    #[arg(long)]
    pub steps: Option<String>,

    /// 所属模块 ID（0=根）
    #[arg(long)]
    pub module: Option<String>,

    /// 所属项目 ID
    #[arg(long)]
    pub project: Option<String>,

    /// 严重程度 1-4
    #[arg(long)]
    pub severity: Option<String>,

    /// 优先级 0-4
    #[arg(long)]
    pub pri: Option<String>,

    /// 指派给（账号）
    #[arg(long)]
    pub assigned_to: Option<String>,

    /// 影响版本 Build
    #[arg(long)]
    pub opened_build: Option<String>,

    /// 截止日期 YYYY-MM-DD
    #[arg(long)]
    pub deadline: Option<String>,

    /// 关键词
    #[arg(long)]
    pub keywords: Option<String>,

    /// Bug 类型（codeerror/designchange/newfeature/others/...）
    #[arg(long, value_name = "TYPE")]
    pub r#type: Option<String>,

    /// 操作系统（all/windows/win10/android/ios/...）
    #[arg(long)]
    pub os: Option<String>,

    /// 浏览器（all/ie/chrome/firefox/...）
    #[arg(long)]
    pub browser: Option<String>,

    /// 抄送账号（可多次）
    #[arg(long)]
    pub mailto: Vec<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct EditArgs {
    /// Bug ID
    pub id: String,

    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub steps: Option<String>,
    /// 严重程度 1-4
    #[arg(long)]
    pub severity: Option<String>,
    /// 优先级 0-4
    #[arg(long)]
    pub pri: Option<String>,
    /// 指派人账号
    #[arg(long)]
    pub assigned_to: Option<String>,
    /// 状态（active/resolved/closed）
    #[arg(long)]
    pub status: Option<String>,
    /// 解决方案（fixed/bydesign/duplicate/postponed/willnotfix/...）
    #[arg(long)]
    pub resolution: Option<String>,
    /// 解决版本 Build
    #[arg(long)]
    pub resolved_build: Option<String>,
    /// 影响版本 Build（编辑表单必填，缺省用当前值）
    #[arg(long)]
    pub opened_build: Option<String>,
    #[arg(long)]
    pub deadline: Option<String>,
    #[arg(long)]
    pub keywords: Option<String>,
    #[arg(long, value_name = "TYPE")]
    pub r#type: Option<String>,
    #[arg(long)]
    pub os: Option<String>,
    #[arg(long)]
    pub browser: Option<String>,
    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct AssignArgs {
    /// Bug ID
    pub id: String,
    /// 指派给（账号）
    pub account: String,
    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct ResolveArgs {
    /// Bug ID
    pub id: String,

    /// 解决方案（fixed/bydesign/duplicate/postponed/willnotfix/notrepro/...）
    #[arg(long)]
    pub resolution: Option<String>,

    /// 解决版本 Build
    #[arg(long)]
    pub resolved_build: Option<String>,

    /// 新建 Build 名称（与 --resolved-build 配合）
    #[arg(long)]
    pub build_name: Option<String>,

    /// 解决后指派给（账号）
    #[arg(long)]
    pub assigned_to: Option<String>,

    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct ActivateArgs {
    /// Bug ID
    pub id: String,

    #[arg(long)]
    pub assigned_to: Option<String>,
    #[arg(long)]
    pub opened_build: Option<String>,
    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct CloseArgs {
    /// Bug ID
    pub id: String,

    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct ConfirmArgs {
    /// Bug ID
    pub id: String,

    #[arg(long)]
    pub comment: Option<String>,

    #[command(flatten)]
    pub write: WriteFlags,
}

#[derive(Debug, Clone, Args)]
pub struct CommentArgs {
    /// Bug ID
    pub id: String,

    /// 评论内容
    #[arg(long)]
    pub comment: String,

    #[command(flatten)]
    pub write: WriteFlags,
}

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

pub async fn handle(args: BugArgs, ctx: &CommandContext) -> ExitCode {
    let client = match load_session_client(&ctx.profile) {
        Ok(c) => c,
        Err(e) => return fail(&e),
    };
    let gateway = ZentaoV9BugGateway::new(client);

    match args.command {
        BugCommands::List(list) => {
            match gateway
                .list_bugs(BugQuery {
                    assigned_to: list.assigned_to.filter(|s| !s.trim().is_empty()),
                    status: list.status.filter(|s| !s.trim().is_empty()),
                    ..Default::default()
                })
                .await
            {
                Ok(page) => output::print_bug_page(&page, ctx.format)
                    .map_err(|e| ZentaoError::Internal(e.to_string()))
                    .map_or_else(|e| fail(&e), |_| ok()),
                Err(e) => fail(&e.into()),
            }
        }
        BugCommands::Get(get) => match gateway.get_bug(EntityId::from(get.id)).await {
            Ok(detail) => output::print_value(&detail, ctx.format)
                .map_err(|e| ZentaoError::Internal(e.to_string()))
                .map_or_else(|e| fail(&e), |_| ok()),
            Err(e) => fail(&e.into()),
        },
        BugCommands::Create(a) => {
            if a.title.trim().is_empty() {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter("Bug 标题不能为空".into()),
                ));
            }
            let draft = BugDraft {
                title: a.title.clone(),
                steps: a.steps.clone(),
                module: a.module.clone(),
                project: a.project.clone(),
                severity: a.severity.clone(),
                pri: a.pri.clone(),
                assigned_to: a.assigned_to.clone(),
                opened_build: a.opened_build.clone(),
                deadline: a.deadline.clone(),
                keywords: a.keywords.clone(),
                bug_type: a.r#type.clone(),
                os: a.os.clone(),
                browser: a.browser.clone(),
                mailto: a.mailto.clone(),
            };
            let s = summary(
                &format!("创建 Bug（产品 {}）", a.product),
                &[
                    ("title", draft.title.as_str()),
                    ("steps", draft.steps.as_deref().unwrap_or("")),
                    ("severity", draft.severity.as_deref().unwrap_or("")),
                    ("pri", draft.pri.as_deref().unwrap_or("")),
                    ("type", draft.bug_type.as_deref().unwrap_or("")),
                    ("os", draft.os.as_deref().unwrap_or("")),
                    ("browser", draft.browser.as_deref().unwrap_or("")),
                    ("deadline", draft.deadline.as_deref().unwrap_or("")),
                    ("keywords", draft.keywords.as_deref().unwrap_or("")),
                    ("module", draft.module.as_deref().unwrap_or("")),
                    ("project", draft.project.as_deref().unwrap_or("")),
                    ("assignedTo", draft.assigned_to.as_deref().unwrap_or("")),
                    ("openedBuild", draft.opened_build.as_deref().unwrap_or("")),
                ],
            );
            try_write!(
                a.write,
                s,
                gateway.create_bug(EntityId::from(a.product.as_str()), draft.clone())
            )
        }
        BugCommands::Edit(a) => {
            let edit = BugEdit {
                title: a.title.clone(),
                steps: a.steps.clone(),
                severity: a.severity.clone(),
                pri: a.pri.clone(),
                assigned_to: a.assigned_to.clone(),
                status: a.status.clone(),
                resolution: a.resolution.clone(),
                resolved_build: a.resolved_build.clone(),
                opened_build: a.opened_build.clone(),
                deadline: a.deadline.clone(),
                keywords: a.keywords.clone(),
                bug_type: a.r#type.clone(),
                os: a.os.clone(),
                browser: a.browser.clone(),
                comment: a.comment.clone(),
            };
            if all_none(&edit) {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter("未提供任何要修改的字段".into()),
                ));
            }
            let s = summary(
                &format!("编辑 Bug {}", a.id),
                &[
                    ("title", edit.title.as_deref().unwrap_or("")),
                    ("steps", edit.steps.as_deref().unwrap_or("")),
                    ("severity", edit.severity.as_deref().unwrap_or("")),
                    ("pri", edit.pri.as_deref().unwrap_or("")),
                    ("assignedTo", edit.assigned_to.as_deref().unwrap_or("")),
                    ("status", edit.status.as_deref().unwrap_or("")),
                    ("resolution", edit.resolution.as_deref().unwrap_or("")),
                    (
                        "resolvedBuild",
                        edit.resolved_build.as_deref().unwrap_or(""),
                    ),
                    ("openedBuild", edit.opened_build.as_deref().unwrap_or("")),
                    ("deadline", edit.deadline.as_deref().unwrap_or("")),
                    ("keywords", edit.keywords.as_deref().unwrap_or("")),
                    ("type", edit.bug_type.as_deref().unwrap_or("")),
                    ("os", edit.os.as_deref().unwrap_or("")),
                    ("browser", edit.browser.as_deref().unwrap_or("")),
                    ("comment", edit.comment.as_deref().unwrap_or("")),
                ],
            );
            let s =
                format!("{s}  备注：将连当前字段基线一并提交（旧版接口行为，空提交会清空字段）");
            try_write!(
                a.write,
                s,
                gateway.edit_bug(EntityId::from(a.id.as_str()), edit.clone())
            )
        }
        BugCommands::Assign(a) => {
            let edit = BugEdit {
                assigned_to: Some(a.account.clone()),
                comment: a.comment.clone(),
                ..Default::default()
            };
            let s = summary(
                &format!("指派 Bug {}", a.id),
                &[
                    ("assignedTo", a.account.as_str()),
                    ("comment", a.comment.as_deref().unwrap_or("")),
                ],
            );
            try_write!(
                a.write,
                s,
                gateway.edit_bug(EntityId::from(a.id.as_str()), edit.clone())
            )
        }
        BugCommands::Resolve(a) => {
            let p = BugResolveParams {
                resolution: a.resolution.clone(),
                resolved_build: a.resolved_build.clone(),
                build_name: a.build_name.clone(),
                assigned_to: a.assigned_to.clone(),
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("解决 Bug {}", a.id),
                &[
                    ("resolution", p.resolution.as_deref().unwrap_or("")),
                    ("resolvedBuild", p.resolved_build.as_deref().unwrap_or("")),
                    ("buildName", p.build_name.as_deref().unwrap_or("")),
                    ("assignedTo", p.assigned_to.as_deref().unwrap_or("")),
                    ("comment", p.comment.as_deref().unwrap_or("")),
                ],
            );
            try_write!(
                a.write,
                s,
                gateway.resolve_bug(EntityId::from(a.id.as_str()), p.clone())
            )
        }
        BugCommands::Activate(a) => {
            let p = BugActivateParams {
                assigned_to: a.assigned_to.clone(),
                opened_build: a.opened_build.clone(),
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("激活 Bug {}", a.id),
                &[
                    ("assignedTo", p.assigned_to.as_deref().unwrap_or("")),
                    ("openedBuild", p.opened_build.as_deref().unwrap_or("")),
                    ("comment", p.comment.as_deref().unwrap_or("")),
                ],
            );
            try_write!(
                a.write,
                s,
                gateway.activate_bug(EntityId::from(a.id.as_str()), p.clone())
            )
        }
        BugCommands::Close(a) => {
            let p = BugNoteParams {
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("关闭 Bug {}", a.id),
                &[("comment", p.comment.as_deref().unwrap_or(""))],
            );
            try_write!(
                a.write,
                s,
                gateway.close_bug(EntityId::from(a.id.as_str()), p.clone())
            )
        }
        BugCommands::Confirm(a) => {
            let p = BugNoteParams {
                comment: a.comment.clone(),
            };
            let s = summary(
                &format!("确认 Bug {}", a.id),
                &[("comment", p.comment.as_deref().unwrap_or(""))],
            );
            try_write!(
                a.write,
                s,
                gateway.confirm_bug(EntityId::from(a.id.as_str()), p.clone())
            )
        }
        BugCommands::Comment(a) => {
            if a.comment.trim().is_empty() {
                return fail(&ZentaoError::Query(
                    crate::domain::QueryError::InvalidParameter("评论内容不能为空".into()),
                ));
            }
            let s = summary(
                &format!("评论 Bug {}", a.id),
                &[("comment", a.comment.as_str())],
            );
            try_write!(
                a.write,
                s,
                gateway.comment_bug(EntityId::from(a.id.as_str()), &a.comment)
            )
        }
    }
}

fn all_none(edit: &BugEdit) -> bool {
    edit.title.is_none()
        && edit.steps.is_none()
        && edit.severity.is_none()
        && edit.pri.is_none()
        && edit.assigned_to.is_none()
        && edit.status.is_none()
        && edit.resolution.is_none()
        && edit.resolved_build.is_none()
        && edit.opened_build.is_none()
        && edit.deadline.is_none()
        && edit.keywords.is_none()
        && edit.bug_type.is_none()
        && edit.os.is_none()
        && edit.browser.is_none()
        && edit.comment.is_none()
}
