use std::process::ExitCode;

use clap::Args;

/// 登录参数。
#[derive(Debug, Args)]
pub struct LoginArgs {
    /// 禅道服务器地址
    #[arg(short, long)]
    pub server: Option<String>,

    /// 登录账号
    #[arg(short, long)]
    pub account: Option<String>,
}

pub async fn handle(_args: LoginArgs) -> ExitCode {
    // 子任务 03 实现真实登录。
    crate::cli::commands::not_implemented()
}
