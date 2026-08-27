use std::io::IsTerminal;

/// 是否启用 ANSI 着色：stdout 是终端且未设置 NO_COLOR。
pub fn use_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

const RESET: &str = "\x1b[0m";

fn wrap(code: &str, text: &str, enabled: bool) -> String {
    if enabled {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

pub fn green(text: &str) -> String {
    wrap("\x1b[32m", text, use_color())
}

pub fn red(text: &str) -> String {
    wrap("\x1b[31m", text, use_color())
}

pub fn yellow(text: &str) -> String {
    wrap("\x1b[33m", text, use_color())
}

pub fn cyan(text: &str) -> String {
    wrap("\x1b[36m", text, use_color())
}

pub fn dim(text: &str) -> String {
    wrap("\x1b[2m", text, use_color())
}

pub fn bold(text: &str) -> String {
    wrap("\x1b[1m", text, use_color())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_applies_ansi_when_enabled() {
        assert_eq!(wrap("\x1b[32m", "ok", true), "\x1b[32mok\x1b[0m");
    }

    #[test]
    fn wrap_returns_plain_when_disabled() {
        assert_eq!(wrap("\x1b[32m", "ok", false), "ok");
    }

    #[test]
    fn use_color_is_false_when_no_color_is_set() {
        // 测试环境 stdout 非终端，且此处不设置 NO_COLOR，也应返回 false。
        let before = std::env::var_os("NO_COLOR");
        std::env::set_var("NO_COLOR", "1");
        assert!(!use_color());
        match before {
            Some(v) => std::env::set_var("NO_COLOR", v),
            None => std::env::remove_var("NO_COLOR"),
        }
    }
}
