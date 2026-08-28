use std::process::ExitCode;

use clap::Args;

use super::{fail, ok, CommandContext};
use crate::cli::confirm::{confirm_write, WriteControl, WriteFlags};
use crate::cli::style;
use crate::domain::ZentaoError;
use crate::infrastructure::updater;

#[derive(Debug, Clone, Args)]
pub struct UpdateArgs {
    /// 目标版本（默认最新，如 v0.1.2；可指定旧版本降级）
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,

    /// GitHub 仓库（owner/repo）
    #[arg(long, default_value = updater::DEFAULT_REPO, value_name = "OWNER/REPO")]
    pub repo: String,

    /// 只检查是否有新版本，不下载
    #[arg(long)]
    pub check: bool,

    #[command(flatten)]
    pub write: WriteFlags,
}

pub async fn handle(args: UpdateArgs, _ctx: &CommandContext) -> ExitCode {
    match run(args).await {
        Ok(()) => ok(),
        Err(msg) => fail(&ZentaoError::Internal(msg)),
    }
}

async fn run(args: UpdateArgs) -> Result<(), String> {
    let current = updater::CURRENT_VERSION;
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("初始化 HTTP 客户端失败: {e}"))?;

    let tag = match args.version.clone() {
        Some(v) => updater::normalize_tag(&v),
        None => updater::latest_tag(&client, &args.repo).await?,
    };
    let target_version = tag.trim_start_matches('v');

    if args.check {
        if updater::is_newer(target_version, current) {
            println!("有新版本：{target_version}（当前 {current}）→ 运行 `zentao-cli update` 更新");
        } else {
            println!("已是最新版本 {current}");
        }
        return Ok(());
    }

    // 未显式指定版本且已是最新：不下载
    if args.version.is_none() && !updater::is_newer(target_version, current) {
        println!("{}", style::green(&format!("已是最新版本 {current}")));
        return Ok(());
    }

    let target = updater::asset_target().ok_or_else(|| {
        "当前平台不支持一键更新（仅支持 Linux x86_64 / macOS arm64 / Windows x86_64）".to_string()
    })?;
    let base = format!(
        "{}/{}/releases/download/{tag}",
        updater::release_download_base(),
        args.repo
    );
    let summary = crate::cli::output::render_summary(
        "更新 zentao-cli",
        &[
            ("当前版本", current),
            ("目标版本", target_version),
            ("平台资产", &target.asset),
            ("下载地址", &format!("{base}/{}", target.asset)),
        ],
    );
    match confirm_write(&summary, args.write) {
        Ok(WriteControl::Aborted) => return Ok(()),
        Ok(WriteControl::Proceed) => {}
        Err(e) => return Err(e.to_string()),
    }

    let temp = std::env::temp_dir().join(format!("zentao-cli-update-{}", std::process::id()));
    std::fs::create_dir_all(&temp).map_err(|e| format!("创建临时目录失败: {e}"))?;
    let new_bin = updater::fetch_and_extract(&client, &args.repo, &tag, &temp).await;
    let new_bin = new_bin?;
    let replaced = updater::replace_current_exe(&new_bin);
    let _ = std::fs::remove_dir_all(&temp);
    replaced?;

    println!("{}", style::green(&format!("已更新到 {target_version}")));
    Ok(())
}
