use std::process::ExitCode;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct BugArgs {
    #[command(subcommand)]
    pub command: BugCommands,
}

#[derive(Debug, Subcommand)]
pub enum BugCommands {
    /// 列出 Bug
    List(ListArgs),
    /// 查看 Bug 详情
    Get(GetArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// 指派给
    #[arg(short, long)]
    pub assigned_to: Option<String>,
    /// 状态过滤
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Bug ID
    pub id: String,
}

pub async fn handle(_args: BugArgs) -> ExitCode {
    crate::cli::commands::not_implemented()
}
