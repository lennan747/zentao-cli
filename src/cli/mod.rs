pub mod commands;
pub mod output;

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
        Commands::Project(cmd) => commands::project::handle(cmd, &ctx).await,
        Commands::Task(cmd) => commands::task::handle(cmd, &ctx).await,
        Commands::Bug(cmd) => commands::bug::handle(cmd, &ctx).await,
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
            _ => 6,
        },
        ZentaoError::Internal(_) => 7,
    };
    ExitCode::from(code)
}

/// 打印错误到 stderr。
pub fn report_error(err: &crate::domain::ZentaoError) {
    error!(error = %err, "command failed");
    eprintln!("error: {err}");
}
