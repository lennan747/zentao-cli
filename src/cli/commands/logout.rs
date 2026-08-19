use std::process::ExitCode;

use clap::Args;

use super::{fail, ok, CommandContext};
use crate::domain::ZentaoError;
use crate::infrastructure::session::StoredSession;

#[derive(Debug, Args)]
pub struct LogoutArgs {}

/// 只删除本地会话文件；不调用远端退出接口（远端行为未确认，见子任务 03）。
pub async fn handle(_args: LogoutArgs, ctx: &CommandContext) -> ExitCode {
    let path = StoredSession::session_path(&ctx.profile);
    match StoredSession::remove(&path) {
        Ok(true) => {
            println!("已退出登录（profile: {}）", ctx.profile);
            ok()
        }
        Ok(false) => {
            println!("当前没有已保存的会话");
            ok()
        }
        Err(e) => fail(&ZentaoError::Internal(format!("删除会话失败: {e}"))),
    }
}
