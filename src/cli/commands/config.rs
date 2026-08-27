use std::process::ExitCode;

use clap::{Args, Subcommand};
use serde::Serialize;

use super::{fail, ok, CommandContext};
use crate::cli::output;
use crate::cli::style;
use crate::domain::ZentaoError;
use crate::infrastructure::config::Config;

/// 配置管理参数。
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// 显示配置文件路径
    Path,
    /// 显示当前配置
    Show,
    /// 设置登录参数（server / account / timeout）
    Set(SetArgs),
    /// 初始化配置文件模板（已存在则不改动）
    Init,
}

#[derive(Debug, Args)]
pub struct SetArgs {
    /// 配置键：server / account / timeout（秒）
    pub key: String,

    /// 配置值
    pub value: String,
}

/// 供 `config show` 输出的视图（不含密码等敏感信息）。
#[derive(Debug, Serialize)]
struct ConfigView {
    path: String,
    default_profile: String,
    profile: String,
    server: String,
    account: String,
    timeout_seconds: u64,
}

pub async fn handle(args: ConfigArgs, ctx: &CommandContext) -> ExitCode {
    let config_path = Config::config_path();
    match args.command {
        ConfigCommands::Path => {
            println!("{}", config_path.display());
            ok()
        }
        ConfigCommands::Show => {
            let config = match Config::load(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    return fail(&ZentaoError::Internal(format!("读取配置失败: {e}")));
                }
            };
            let profile = config
                .profiles
                .get(&ctx.profile)
                .cloned()
                .unwrap_or_default();
            let view = ConfigView {
                path: config_path.display().to_string(),
                default_profile: config.default_profile.clone(),
                profile: ctx.profile.clone(),
                server: profile.server,
                account: profile.account,
                timeout_seconds: profile.timeout_seconds,
            };
            match output::print_value(&view, ctx.format) {
                Ok(()) => ok(),
                Err(e) => fail(&ZentaoError::Internal(e.to_string())),
            }
        }
        ConfigCommands::Set(set) => {
            let mut config = match Config::load(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    return fail(&ZentaoError::Internal(format!("读取配置失败: {e}")));
                }
            };
            let key = set.key.trim().to_lowercase();
            let profile = config.profiles.entry(ctx.profile.clone()).or_default();

            let report = match key.as_str() {
                "server" => {
                    profile.server = set.value.trim().trim_end_matches('/').to_string();
                    format!("server = {}", profile.server)
                }
                "account" => {
                    profile.account = set.value.trim().to_string();
                    format!("account = {}", profile.account)
                }
                "timeout" => {
                    let seconds: u64 = match set.value.trim().parse() {
                        Ok(v) => v,
                        Err(_) => {
                            return fail(&ZentaoError::Query(
                                crate::domain::QueryError::InvalidParameter(
                                    "timeout 必须是秒为单位的非负整数".into(),
                                ),
                            ));
                        }
                    };
                    profile.timeout_seconds = seconds;
                    format!("timeout_seconds = {seconds}")
                }
                other => {
                    return fail(&ZentaoError::Query(
                        crate::domain::QueryError::InvalidParameter(format!(
                            "不支持的配置键: {other}（可用: server / account / timeout）"
                        )),
                    ));
                }
            };

            if config.default_profile.is_empty() {
                config.default_profile = ctx.profile.clone();
            }
            if let Err(e) = config.save(&config_path) {
                return fail(&ZentaoError::Internal(format!("保存配置失败: {e}")));
            }
            println!(
                "{}",
                style::green(&format!("已设置 {report}（profile: {}）", ctx.profile))
            );
            ok()
        }
        ConfigCommands::Init => {
            if config_path.exists() {
                println!(
                    "{}",
                    style::dim(&format!("配置已存在: {}", config_path.display()))
                );
                return ok();
            }
            let config = Config::default();
            if let Err(e) = config.save(&config_path) {
                return fail(&ZentaoError::Internal(format!("保存配置失败: {e}")));
            }
            println!(
                "{}",
                style::green(&format!("已初始化配置: {}", config_path.display()))
            );
            ok()
        }
    }
}
