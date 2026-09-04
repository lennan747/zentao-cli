pub mod bug;
pub mod config;
pub mod login;
pub mod logout;
pub mod project;
pub mod task;
pub mod update;
pub mod user;

use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};

use crate::adapters::zentao_v9::{ZentaoV9AuthGateway, ZentaoV9Client};
use crate::application::{AuthGateway, Credentials};
use crate::domain::{AuthError, ZentaoError};
use crate::infrastructure::config::Config;
use crate::infrastructure::session::StoredSession;

/// 禅道 CLI
#[derive(Debug, Parser)]
#[command(name = "zentao-cli")]
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
    /// 配置文件管理（server/account/timeout/password）
    Config(config::ConfigArgs),
    /// 项目查询
    Project(project::ProjectArgs),
    /// 任务查询与写操作
    Task(task::TaskArgs),
    /// Bug 查询与写操作
    Bug(bug::BugArgs),
    /// 用户查询（账号 → 真实姓名）
    User(user::UserArgs),
    /// 更新 zentao-cli 到最新版本
    Update(update::UpdateArgs),
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

    let client = build_client(&session.server, profile_timeout(profile))?;
    client
        .import_cookies(&session.cookie)
        .map_err(|_| ZentaoError::Auth(AuthError::Other("会话文件损坏，请重新登录".into())))?;
    Ok(client)
}

/// 读取 profile 配置的请求超时（秒）；0 或未配置表示用默认超时。
pub fn profile_timeout(profile: &str) -> u64 {
    Config::load(Config::config_path())
        .ok()
        .and_then(|c| c.profiles.get(profile).map(|p| p.timeout_seconds))
        .unwrap_or(0)
}

/// 按超时配置创建 HTTP 客户端。
pub fn build_client(server: &str, timeout_seconds: u64) -> Result<ZentaoV9Client, ZentaoError> {
    let result = if timeout_seconds > 0 {
        ZentaoV9Client::with_timeout(server, Duration::from_secs(timeout_seconds))
    } else {
        ZentaoV9Client::new(server)
    };
    result.map_err(|e| ZentaoError::Internal(format!("创建 HTTP 客户端失败: {e}")))
}

/// 用配置中保存的 server/account/password 静默重登，刷新会话文件。
/// 配置缺少任一凭据时返回错误（调用方应保持原失败结果）。
pub async fn relogin(profile: &str) -> Result<(), ZentaoError> {
    let config_path = Config::config_path();
    let config = Config::load(&config_path)
        .map_err(|e| ZentaoError::Internal(format!("读取配置失败: {e}")))?;
    let saved = config.profiles.get(profile).cloned().unwrap_or_default();

    let server = saved.server.trim_end_matches('/').to_string();
    let account = saved.account.clone();
    let password = saved.password.clone().filter(|p| !p.is_empty());

    if server.is_empty() || account.is_empty() || password.is_none() {
        return Err(ZentaoError::Auth(AuthError::Other(
            "会话已过期，且配置缺少 server/account/password，无法自动重登，请先执行 zentao login"
                .into(),
        )));
    }

    let client = build_client(&server, saved.timeout_seconds)?;
    let gateway = ZentaoV9AuthGateway::new(client);
    let session = gateway
        .login(&Credentials {
            account: account.clone(),
            password: password.expect("password checked non-empty above"),
        })
        .await
        .map_err(ZentaoError::Auth)?;

    let stored = StoredSession {
        server: session.server.clone(),
        cookie: session.cookie,
    };
    stored
        .save(StoredSession::session_path(profile))
        .map_err(|e| ZentaoError::Internal(format!("保存会话失败: {e}")))
}
