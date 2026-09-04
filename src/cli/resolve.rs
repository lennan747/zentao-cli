//! 指派人解析：姓名/模糊输入 → 禅道账号。
//!
//! 规则（2026-09-04 用户确认）：账号精确 → 姓名精确 → 账号/姓名包含（大小写不敏感，
//! 见 `domain::match_users`）。唯一命中自动采用；多候选在 TTY 下编号选择、非 TTY 报错列候选
//! （退出码 6）；0 候选报错并给相近建议。用户列表获取失败时 ASCII 输入按账号直通（保持
//! 旧行为），非 ASCII 输入报错。

use std::io::IsTerminal;
use std::process::ExitCode;

use crate::adapters::zentao_v9::ZentaoV9UserGateway;
use crate::application::UserGateway;
use crate::cli::commands::{fail, ok};
use crate::cli::picker;
use crate::cli::style;
use crate::domain::{
    display_mapping, match_users, QueryError, UserMatch, UserSummary, ZentaoError,
};

/// 解析结果：`account` 为提交表单值，`display` 为确认摘要展示值（含映射过程）。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUser {
    pub account: String,
    pub display: String,
}

#[derive(Debug)]
enum Resolution {
    Unique(UserSummary),
    Multiple(Vec<UserSummary>),
    Passthrough,
}

fn candidate_label(user: &UserSummary) -> String {
    if user.realname.is_empty() {
        user.account.clone()
    } else {
        format!("{}（{}）", user.account, user.realname)
    }
}

fn format_candidates(candidates: &[UserSummary]) -> String {
    candidates
        .iter()
        .map(candidate_label)
        .collect::<Vec<_>>()
        .join("、")
}

/// 纯判定：输入 + 用户列表（可能获取失败）→ 解析分支。
fn classify(
    raw: &str,
    users: &Result<Vec<UserSummary>, QueryError>,
) -> Result<Resolution, ZentaoError> {
    let list = match users {
        Ok(list) => list,
        Err(e) => {
            if raw.is_ascii() {
                eprintln!(
                    "{}",
                    style::dim(&format!(
                        "warning: 无法获取用户列表（{e}），按账号原样提交: {raw}"
                    ))
                );
                return Ok(Resolution::Passthrough);
            }
            return Err(ZentaoError::Query(QueryError::InvalidParameter(format!(
                "无法获取用户列表来解析姓名（{e}）；请改用账号精确指派，可用 zentao-cli user search 查询账号"
            ))));
        }
    };

    match match_users(raw, list) {
        UserMatch::Unique(user) => Ok(Resolution::Unique(user)),
        UserMatch::Multiple(candidates) => Ok(Resolution::Multiple(candidates)),
        UserMatch::None { suggestions } => {
            let hint = if suggestions.is_empty() {
                String::new()
            } else {
                format!("，相近候选: {}", format_candidates(&suggestions))
            };
            Err(ZentaoError::Query(QueryError::InvalidParameter(format!(
                "未找到用户 \"{raw}\"{hint}；可用 zentao-cli user search 查询账号"
            ))))
        }
    }
}

/// 解析写命令的指派人输入。
///
/// 返回值约定：
/// - `Ok(None)`：未提供输入，调用方按"未指定"原逻辑继续；
/// - `Ok(Some(resolved))`：解析成功；
/// - `Err(code)`：调用方应直接 `return code`——解析失败已打印错误（`fail`），
///   或用户在 TTY 取消了选择（`ok()`，视为正常退出，不提交任何变更）。
pub async fn assigned_to(
    user_gateway: &ZentaoV9UserGateway,
    raw: Option<&str>,
) -> Result<Option<ResolvedUser>, ExitCode> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };

    let users = user_gateway.list_users().await;
    let resolution = match classify(raw, &users) {
        Ok(resolution) => resolution,
        Err(e) => return Err(fail(&e)),
    };

    let resolved = match resolution {
        Resolution::Passthrough => ResolvedUser {
            account: raw.to_string(),
            display: raw.to_string(),
        },
        Resolution::Unique(user) => ResolvedUser {
            account: user.account.clone(),
            display: display_mapping(raw, &user),
        },
        Resolution::Multiple(candidates) => {
            if !std::io::stdin().is_terminal() {
                return Err(fail(&ZentaoError::Query(QueryError::InvalidParameter(
                    format!(
                        "找到多个匹配 \"{raw}\" 的用户: {}；非交互环境请用账号精确指定",
                        format_candidates(&candidates)
                    ),
                ))));
            }
            let options: Vec<String> = candidates.iter().map(candidate_label).collect();
            match picker::pick_candidate(
                &format!("找到多个匹配 \"{raw}\" 的用户，请选择指派人:"),
                &options,
            ) {
                Ok(Some(index)) => {
                    let user = &candidates[index];
                    ResolvedUser {
                        account: user.account.clone(),
                        display: display_mapping(raw, user),
                    }
                }
                Ok(None) => {
                    eprintln!("{}", style::dim("已取消选择，未提交任何变更"));
                    return Err(ok());
                }
                Err(e) => return Err(fail(&e)),
            }
        }
    };
    Ok(Some(resolved))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(account: &str, realname: &str) -> UserSummary {
        UserSummary {
            account: account.to_string(),
            realname: realname.to_string(),
        }
    }

    fn sample_list() -> Result<Vec<UserSummary>, QueryError> {
        Ok(vec![
            user("wangli", "王力"),
            user("wanglinan", "王李男"),
            user("demo-user", "演示用户"),
        ])
    }

    #[test]
    fn unique_match_resolves_account_and_display() {
        match classify("王李男", &sample_list()) {
            Ok(Resolution::Unique(u)) => assert_eq!(u.account, "wanglinan"),
            other => panic!("expected Unique, got {other:?}"),
        }
    }

    #[test]
    fn multiple_match_returns_candidates() {
        match classify("王", &sample_list()) {
            Ok(Resolution::Multiple(c)) => assert_eq!(c.len(), 2),
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn no_match_error_contains_suggestions() {
        let err = classify("李娜娜", &sample_list()).unwrap_err();
        match err {
            ZentaoError::Query(QueryError::InvalidParameter(msg)) => {
                assert!(msg.contains("未找到用户 \"李娜娜\""));
                assert!(msg.contains("相近候选"));
                assert!(msg.contains("wanglinan（王李男）"));
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn list_failure_with_ascii_input_passes_through() {
        let users = Err(QueryError::Remote("网络不可达".into()));
        match classify("demo-user", &users) {
            Ok(Resolution::Passthrough) => {}
            other => panic!("expected Passthrough, got {other:?}"),
        }
    }

    #[test]
    fn list_failure_with_non_ascii_input_errors() {
        let users = Err(QueryError::Remote("网络不可达".into()));
        let err = classify("王李男", &users).unwrap_err();
        match err {
            ZentaoError::Query(QueryError::InvalidParameter(msg)) => {
                assert!(msg.contains("无法获取用户列表"));
                assert!(msg.contains("账号精确指派"));
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn candidate_label_omits_empty_realname() {
        assert_eq!(candidate_label(&user("admin", "")), "admin");
        assert_eq!(candidate_label(&user("wangli", "王力")), "wangli（王力）");
        assert_eq!(
            format_candidates(&[user("wangli", "王力"), user("admin", "")]),
            "wangli（王力）、admin"
        );
    }
}
