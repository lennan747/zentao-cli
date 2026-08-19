pub mod bug;
pub mod login;
pub mod logout;
pub mod project;
pub mod task;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::adapters::zentao_v9::ZentaoV9Client;
use crate::domain::{AuthError, ZentaoError};
use crate::infrastructure::session::StoredSession;

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

/// 传递给子命令的运行上下文。
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub profile: String,
    pub format: OutputFormat,
}

pub fn ok() -> ExitCode {
    ExitCode::SUCCESS
}

/// 报告错误并返回对应退出码。
pub fn fail(err: &ZentaoError) -> ExitCode {
    crate::cli::report_error(err);
    crate::cli::exit_code_from_error(err)
}

/// 从本地会话文件恢复已登录客户端。
pub fn load_session_client(profile: &str) -> Result<ZentaoV9Client, ZentaoError> {
    let path = StoredSession::session_path(profile);
    let session = StoredSession::load(&path)
        .map_err(|e| ZentaoError::Internal(format!("读取会话失败: {e}")))?
        .ok_or_else(|| {
            ZentaoError::Auth(AuthError::Other("尚未登录，请先执行 zentao login".into()))
        })?;

    let client = ZentaoV9Client::new(&session.server)
        .map_err(|e| ZentaoError::Internal(format!("创建 HTTP 客户端失败: {e}")))?;
    client
        .import_cookies(&session.cookie)
        .map_err(|_| ZentaoError::Auth(AuthError::Other("会话文件损坏，请重新登录".into())))?;
    Ok(client)
}
