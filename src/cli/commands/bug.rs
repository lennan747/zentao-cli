use std::process::ExitCode;

use clap::{Args, Subcommand};

use super::{fail, load_session_client, ok, CommandContext};
use crate::adapters::zentao_v9::ZentaoV9BugGateway;
use crate::application::{BugGateway, BugQuery};
use crate::cli::output;
use crate::domain::{EntityId, ZentaoError};

#[derive(Debug, Args)]
pub struct BugArgs {
    #[command(subcommand)]
    pub command: BugCommands,
}

#[derive(Debug, Subcommand)]
pub enum BugCommands {
    /// 列出指派给我的 Bug
    List(ListArgs),
    /// 查看 Bug 详情
    Get(GetArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// 指派对象（旧版接口仅支持 me）
    #[arg(short, long)]
    pub assigned_to: Option<String>,
    /// 状态过滤（active/resolved/closed），本地过滤
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Bug ID
    pub id: String,
}

pub async fn handle(args: BugArgs, ctx: &CommandContext) -> ExitCode {
    let client = match load_session_client(&ctx.profile) {
        Ok(c) => c,
        Err(e) => return fail(&e),
    };
    let gateway = ZentaoV9BugGateway::new(client);

    let result = match args.command {
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
                    .map_err(|e| ZentaoError::Internal(e.to_string())),
                Err(e) => Err(e.into()),
            }
        }
        BugCommands::Get(get) => match gateway.get_bug(EntityId::from(get.id)).await {
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
