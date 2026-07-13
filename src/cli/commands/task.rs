use std::process::ExitCode;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommands,
}

#[derive(Debug, Subcommand)]
pub enum TaskCommands {
    /// 列出任务
    List(ListArgs),
    /// 查看任务详情
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
    /// 任务 ID
    pub id: String,
}

pub async fn handle(_args: TaskArgs) -> ExitCode {
    crate::cli::commands::not_implemented()
}
