use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use clap::Args;

use super::{fail, ok, CommandContext};
use crate::adapters::zentao_v9::{ZentaoV9AuthGateway, ZentaoV9Client};
use crate::application::{AuthGateway, Credentials};
use crate::domain::{AuthError, ZentaoError};
use crate::infrastructure::config::Config;
use crate::infrastructure::session::StoredSession;

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

fn prompt(label: &str) -> Result<String, ZentaoError> {
    print!("{label}");
    io::stdout()
        .flush()
        .map_err(|e| ZentaoError::Internal(format!("写出错: {e}")))?;
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| ZentaoError::Internal(format!("读入错: {e}")))?;
    Ok(line.trim().to_string())
}

fn internal(msg: String) -> ZentaoError {
    ZentaoError::Internal(msg)
}

pub async fn handle(args: LoginArgs, ctx: &CommandContext) -> ExitCode {
    let config_path = Config::config_path();
    let mut config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => return fail(&internal(format!("读取配置失败: {e}"))),
    };

    let saved = config
        .profiles
        .get(&ctx.profile)
        .cloned()
        .unwrap_or_default();

    let server = match nonempty(args.server).or_else(|| nonempty(Some(saved.server.clone()))) {
        Some(s) => s.trim_end_matches('/').to_string(),
        None => match prompt("禅道服务器地址: ") {
            Ok(s) if !s.is_empty() => s.trim_end_matches('/').to_string(),
            Ok(_) => return fail(&ZentaoError::Auth(AuthError::MissingCredentials)),
            Err(e) => return fail(&e),
        },
    };

    let account = match nonempty(args.account).or_else(|| nonempty(Some(saved.account.clone()))) {
        Some(a) => a,
        None => match prompt("禅道账号: ") {
            Ok(a) if !a.is_empty() => a,
            Ok(_) => return fail(&ZentaoError::Auth(AuthError::MissingCredentials)),
            Err(e) => return fail(&e),
        },
    };

    // 密码只经无回显终端读取；无 TTY（管道/脚本）时退化为普通 stdin 读取，
    // 不进入参数、日志或配置。
    let password = match rpassword::prompt_password("禅道密码: ") {
        Ok(p) => p,
        Err(_) => match prompt("禅道密码: ") {
            Ok(p) if !p.is_empty() => {
                eprintln!("警告: 当前无终端控制，密码以回显方式从标准输入读取");
                p
            }
            Ok(_) => return fail(&ZentaoError::Auth(AuthError::MissingCredentials)),
            Err(e) => return fail(&e),
        },
    };

    let client = match ZentaoV9Client::new(&server) {
        Ok(c) => c,
        Err(e) => return fail(&internal(format!("创建 HTTP 客户端失败: {e}"))),
    };
    let gateway = ZentaoV9AuthGateway::new(client);

    match gateway
        .login(&Credentials {
            account: account.clone(),
            password,
        })
        .await
    {
        Ok(session) => {
            let stored = StoredSession {
                server: session.server.clone(),
                cookie: session.cookie,
            };
            if let Err(e) = stored.save(StoredSession::session_path(&ctx.profile)) {
                return fail(&internal(format!("保存会话失败: {e}")));
            }

            let profile = config.profiles.entry(ctx.profile.clone()).or_default();
            profile.server = session.server;
            profile.account = account;
            if config.default_profile.is_empty() {
                config.default_profile = ctx.profile.clone();
            }
            if let Err(e) = config.save(&config_path) {
                return fail(&internal(format!("保存配置失败: {e}")));
            }

            println!("登录成功: {}", stored.server);
            ok()
        }
        Err(e) => fail(&ZentaoError::Auth(e)),
    }
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}
