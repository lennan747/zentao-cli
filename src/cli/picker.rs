//! 编号候选选择器（TTY 交互）。

use std::io::{self, Write};

use crate::cli::style;
use crate::domain::ZentaoError;

/// 单行选择的解析结果。
#[derive(Debug, PartialEq)]
pub enum Choice {
    /// 选中（0 基下标）。
    Select(usize),
    /// 取消（空输入或 EOF）。
    Cancel,
    /// 非法输入，需要重询。
    Invalid,
}

/// 解析选择输入：1 基编号；空输入视为取消。
pub fn parse_choice(line: &str, len: usize) -> Choice {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Choice::Cancel;
    }
    match trimmed.parse::<usize>() {
        Ok(n) if n >= 1 && n <= len => Choice::Select(n - 1),
        _ => Choice::Invalid,
    }
}

/// 列出编号候选并读取选择；用户取消（空输入/EOF）返回 `Ok(None)`。
pub fn pick_candidate(prompt: &str, options: &[String]) -> Result<Option<usize>, ZentaoError> {
    println!("{prompt}");
    for (i, option) in options.iter().enumerate() {
        println!("  {} {option}", style::cyan(&format!("[{}]", i + 1)));
    }
    loop {
        print!(
            "{} ",
            style::yellow(&format!("请输入编号选择（1-{}，回车取消）:", options.len()))
        );
        io::stdout()
            .flush()
            .map_err(|e| ZentaoError::Internal(format!("写出错: {e}")))?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| ZentaoError::Internal(format!("读入错: {e}")))?;
        match parse_choice(&line, options.len()) {
            Choice::Select(index) => return Ok(Some(index)),
            Choice::Cancel => return Ok(None),
            Choice::Invalid => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_one_based_choice() {
        assert_eq!(parse_choice("1", 3), Choice::Select(0));
        assert_eq!(parse_choice(" 2 ", 3), Choice::Select(1));
        assert_eq!(parse_choice("3\n", 3), Choice::Select(2));
    }

    #[test]
    fn empty_input_cancels() {
        assert_eq!(parse_choice("", 3), Choice::Cancel);
        assert_eq!(parse_choice("   ", 3), Choice::Cancel);
        assert_eq!(parse_choice("\n", 3), Choice::Cancel);
    }

    #[test]
    fn out_of_range_or_non_numeric_is_invalid() {
        assert_eq!(parse_choice("0", 3), Choice::Invalid);
        assert_eq!(parse_choice("4", 3), Choice::Invalid);
        assert_eq!(parse_choice("abc", 3), Choice::Invalid);
        assert_eq!(parse_choice("-1", 3), Choice::Invalid);
        assert_eq!(parse_choice("1.5", 3), Choice::Invalid);
    }

    #[test]
    fn zero_length_options_never_selects() {
        assert_eq!(parse_choice("1", 0), Choice::Invalid);
        assert_eq!(parse_choice("", 0), Choice::Cancel);
    }
}
