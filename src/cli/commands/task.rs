use std::process::ExitCode;

use clap::{Args, Subcommand};

use super::{fail, load_session_client, ok, CommandContext};
use crate::adapters::zentao_v9::ZentaoV9TaskGateway;
use crate::application::{TaskGateway, TaskQuery};
use crate::cli::output;
use crate::domain::{EntityId, ZentaoError};

#[derive(Debug, Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommands,
}

#[derive(Debug, Subcommand)]
pub enum TaskCommands {
    /// 列出我的任务
    List(ListArgs),
    /// 查看任务详情
    Get(GetArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// 指派对象（旧版接口仅支持 me）
    #[arg(short, long)]
    pub assigned_to: Option<String>,
    /// 状态过滤（wait/doing/done/paused/cancel/closed），本地过滤
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// 任务 ID
    pub id: String,
}

pub async fn handle(args: TaskArgs, ctx: &CommandContext) -> ExitCode {
    let client = match load_session_client(&ctx.profile) {
        Ok(c) => c,
        Err(e) => return fail(&e),
    };
    let gateway = ZentaoV9TaskGateway::new(client);

    let result = match args.command {
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
                    .map_err(|e| ZentaoError::Internal(e.to_string())),
                Err(e) => Err(e.into()),
            }
        }
        TaskCommands::Get(get) => match gateway.get_task(EntityId::from(get.id)).await {
            Ok(detail) => output::print_value(&detail, ctx.format)
                .map_err(|e| ZentaoError::Internal(e.to_string())),
            Err(e) => Err(e.into()),
        },
    };

    match result {
        Ok(()) => ok(),
        Err(e) => fail(&e),
    }
}
