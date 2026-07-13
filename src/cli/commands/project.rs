use std::process::ExitCode;

use clap::{Args, Subcommand};

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
    /// 状态过滤
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// 项目 ID
    pub id: String,
}

pub async fn handle(_args: ProjectArgs) -> ExitCode {
    crate::cli::commands::not_implemented()
}
