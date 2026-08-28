pub mod commands;
pub mod confirm;
pub mod output;
pub mod style;

use std::process::ExitCode;

use clap::Parser;
use tracing::error;

use crate::cli::commands::{Cli, Commands};

/// 运行 CLI 并返回退出码。
pub async fn run() -> ExitCode {
    let cli = Cli::parse();

    crate::infrastructure::logging::init(cli.verbose);

    let ctx = commands::CommandContext {
        profile: cli.profile,
        format: cli.format,
    };

    match cli.command {
        Commands::Login(cmd) => commands::login::handle(cmd, &ctx).await,
        Commands::Logout(cmd) => commands::logout::handle(cmd, &ctx).await,
        Commands::Config(cmd) => commands::config::handle(cmd, &ctx).await,
        Commands::Project(cmd) => {
            run_with_session_recovery(&ctx.profile, || {
                commands::project::handle(cmd.clone(), &ctx)
            })
            .await
        }
        Commands::Task(cmd) => {
            run_with_session_recovery(&ctx.profile, || commands::task::handle(cmd.clone(), &ctx))
                .await
        }
        Commands::Bug(cmd) => {
            run_with_session_recovery(&ctx.profile, || commands::bug::handle(cmd.clone(), &ctx))
                .await
        }
    }
}

/// 会话过期（退出码 3）时，若配置含密码则静默重登并重跑一次命令。
///
/// 首次失败的错误信息可能已打印到 stderr；重试成功时命令整体仍返回成功，
/// 并打印一条自动重登提示。
async fn run_with_session_recovery<F, Fut>(profile: &str, run: F) -> ExitCode
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = ExitCode>,
{
    let first = run().await;
    if first != ExitCode::from(3u8) {
        return first;
    }
    if commands::relogin(profile).await.is_ok() {
        eprintln!(
            "{}",
            style::dim("会话过期，已用配置密码自动重登，正在重试…")
        );
        run().await
    } else {
        first
    }
}

/// 将领域错误映射为退出码。
pub fn exit_code_from_error(err: &crate::domain::ZentaoError) -> ExitCode {
    use crate::domain::ZentaoError;
    let code = match err {
        ZentaoError::Auth(_) => 3,
        ZentaoError::Query(q) => match q {
            crate::domain::QueryError::NotFound => 4,
            crate::domain::QueryError::Forbidden => 4,
            crate::domain::QueryError::SessionExpired => 3,
            crate::domain::QueryError::InvalidParameter(_) => 6,
            crate::domain::QueryError::Rejected(_) => 6,
            crate::domain::QueryError::ParseError(_) => 6,
            crate::domain::QueryError::IncompatibleResponse => 6,
            crate::domain::QueryError::Remote(_) => 6,
        },
        ZentaoError::Internal(_) => 7,
    };
    ExitCode::from(code)
}

/// 打印错误到 stderr。
pub fn report_error(err: &crate::domain::ZentaoError) {
    if crate::infrastructure::logging::verbose() {
        error!(error = %err, "command failed");
    }
    eprintln!("{}", crate::cli::style::red(&format!("error: {err}")));
}
