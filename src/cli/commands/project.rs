use std::process::ExitCode;

use clap::{Args, Subcommand};

use super::{fail, load_session_client, ok, CommandContext};
use crate::adapters::zentao_v9::ZentaoV9ProjectGateway;
use crate::application::{ProjectGateway, ProjectQuery};
use crate::cli::output;
use crate::domain::{EntityId, ZentaoError};

#[derive(Debug, Args)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommands,
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommands {
    /// 列出项目
    List(ListArgs),
    /// 查看项目详情
    Get(GetArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// 状态过滤（wait/doing/done/suspended/closed，all 表示全部）
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// 项目 ID
    pub id: String,
}

pub async fn handle(args: ProjectArgs, ctx: &CommandContext) -> ExitCode {
    let client = match load_session_client(&ctx.profile) {
        Ok(c) => c,
        Err(e) => return fail(&e),
    };
    let gateway = ZentaoV9ProjectGateway::new(client);

    let result = match args.command {
        ProjectCommands::List(list) => {
            match gateway
                .list_projects(ProjectQuery {
                    status: list.status.filter(|s| !s.trim().is_empty()),
                    ..Default::default()
                })
                .await
            {
                Ok(page) => output::print_project_page(&page, ctx.format)
                    .map_err(|e| ZentaoError::Internal(e.to_string())),
                Err(e) => Err(e.into()),
            }
        }
        ProjectCommands::Get(get) => match gateway.get_project(EntityId::from(get.id)).await {
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
