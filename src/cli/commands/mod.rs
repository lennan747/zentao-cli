pub mod bug;
pub mod login;
pub mod logout;
pub mod project;
pub mod task;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// 禅道 CLI
#[derive(Debug, Parser)]
#[command(name = "zentao")]
#[command(about = "禅道v9.0.3 命令行客户端")]
#[command(version)]
pub struct Cli {
    /// 使用的 profile
    #[arg(long, global = true, default_value = "default")]
    pub profile: String,

    /// 输出格式
    #[arg(long, global = true, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// 显示更多诊断信息
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 登录禅道
    Login(login::LoginArgs),
    /// 退出登录
    Logout(logout::LogoutArgs),
    /// 项目查询
    Project(project::ProjectArgs),
    /// 任务查询
    Task(task::TaskArgs),
    /// Bug 查询
    Bug(bug::BugArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

pub fn ok() -> ExitCode {
    ExitCode::SUCCESS
}

pub fn not_implemented() -> ExitCode {
    eprintln!("error: command not implemented in skeleton");
    ExitCode::from(2)
}
