use std::io::{self, IsTerminal, Write};

use clap::Args;

use crate::domain::ZentaoError;

/// 写命令共用的安全选项。
#[derive(Debug, Clone, Copy, Default, Args)]
pub struct WriteFlags {
    /// 跳过交互确认直接执行
    #[arg(long)]
    pub yes: bool,

    /// 只显示将提交的内容，不实际执行
    #[arg(long)]
    pub dry_run: bool,
}

/// 交互确认结果。
pub enum WriteControl {
    Proceed,
    Aborted,
}

/// 默认确认流程：
///
/// 1. `--dry-run`：打印摘要后退出不执行；
/// 2. `--yes`：打印摘要后直接执行；
/// 3. 交互终端：打印摘要并询问 y/N；
/// 4. 无终端且没有 `--yes`/`--dry-run`：拒绝执行。
pub fn confirm_write(summary: &str, flags: WriteFlags) -> Result<WriteControl, ZentaoError> {
    println!("{summary}");

    if flags.dry_run {
        println!("\n[dry-run] 未执行，如需执行去掉 --dry-run");
        return Ok(WriteControl::Aborted);
    }
    if flags.yes {
        return Ok(WriteControl::Proceed);
    }
    if !io::stdin().is_terminal() {
        return Err(ZentaoError::Internal(
            "写操作需要交互确认，但当前没有终端；请使用 --yes 或 --dry-run".into(),
        ));
    }

    loop {
        print!("确认执行? [y/N] ");
        io::stdout()
            .flush()
            .map_err(|e| ZentaoError::Internal(format!("写出错: {e}")))?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| ZentaoError::Internal(format!("读入错: {e}")))?;
        match line.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(WriteControl::Proceed),
            "" | "n" | "no" => return Ok(WriteControl::Aborted),
            _ => continue,
        }
    }
}
