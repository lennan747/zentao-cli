use std::process::ExitCode;

use clap::Args;

#[derive(Debug, Args)]
pub struct LogoutArgs {}

pub async fn handle(_args: LogoutArgs) -> ExitCode {
    crate::cli::commands::not_implemented()
}
