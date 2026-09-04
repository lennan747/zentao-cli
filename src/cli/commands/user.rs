use std::process::ExitCode;

use clap::{Args, Subcommand};

use super::{fail, load_session_client, ok, CommandContext};
use crate::adapters::zentao_v9::ZentaoV9UserGateway;
use crate::application::UserGateway;
use crate::cli::output;
use crate::domain::{filter_users, QueryError, ZentaoError};

#[derive(Debug, Clone, Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum UserCommands {
    /// 列出全部用户（账号 → 真实姓名）
    List,
    /// 按账号/姓名关键词搜索用户
    Search(SearchArgs),
}

#[derive(Debug, Clone, Args)]
pub struct SearchArgs {
    /// 关键词（账号或姓名，包含匹配，大小写不敏感）
    pub keyword: String,
}

pub async fn handle(args: UserArgs, ctx: &CommandContext) -> ExitCode {
    let client = match load_session_client(&ctx.profile) {
        Ok(c) => c,
        Err(e) => return fail(&e),
    };
    let gateway = ZentaoV9UserGateway::new(client);

    let result = match args.command {
        UserCommands::List => match gateway.list_users().await {
            Ok(users) => output::print_user_page(&users, ctx.format)
                .map_err(|e| ZentaoError::Internal(e.to_string())),
            Err(e) => Err(e.into()),
        },
        UserCommands::Search(search) => {
            if search.keyword.trim().is_empty() {
                return fail(&ZentaoError::Query(QueryError::InvalidParameter(
                    "搜索关键词不能为空".into(),
                )));
            }
            match gateway.list_users().await {
                Ok(users) => {
                    let hits = filter_users(&search.keyword, &users);
                    output::print_user_page(&hits, ctx.format)
                        .map_err(|e| ZentaoError::Internal(e.to_string()))
                }
                Err(e) => Err(e.into()),
            }
        }
    };

    match result {
        Ok(()) => ok(),
        Err(e) => fail(&e),
    }
}
