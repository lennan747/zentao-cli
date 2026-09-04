/// 禅道用户（账号 → 真实姓名）。
///
/// 旧版接口在几乎所有页面响应的顶层 `users` 字段给出全量映射，
/// 本模块只承载匹配规则，不涉及任何 IO。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserSummary {
    pub account: String,
    pub realname: String,
}

/// 输入解析到用户的结果。
#[derive(Debug, Clone, PartialEq)]
pub enum UserMatch {
    /// 唯一命中。
    Unique(UserSummary),
    /// 多个命中，需要调用方消歧（交互选择或报错列候选）。
    Multiple(Vec<UserSummary>),
    /// 无命中，附带相近候选建议（最多 5 条）。
    None { suggestions: Vec<UserSummary> },
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// 过滤出账号或姓名包含关键词的用户（大小写不敏感）；空关键词返回空列表。
pub fn filter_users(keyword: &str, users: &[UserSummary]) -> Vec<UserSummary> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Vec::new();
    }
    users
        .iter()
        .filter(|u| contains_ci(&u.account, keyword) || contains_ci(&u.realname, keyword))
        .cloned()
        .collect()
}

/// 按规则把输入解析为用户：账号精确 → 姓名精确 → 账号/姓名包含（均大小写不敏感）。
///
/// 空输入返回 `None`（调用方应在入口处过滤空值，这里防御性兜底）。
pub fn match_users(input: &str, users: &[UserSummary]) -> UserMatch {
    let input = input.trim();
    if input.is_empty() {
        return UserMatch::None {
            suggestions: Vec::new(),
        };
    }

    let hits: Vec<UserSummary> = users
        .iter()
        .filter(|u| u.account.eq_ignore_ascii_case(input))
        .cloned()
        .collect();
    if !hits.is_empty() {
        return into_match(hits);
    }

    let lower = input.to_lowercase();
    let hits: Vec<UserSummary> = users
        .iter()
        .filter(|u| !u.realname.is_empty() && u.realname.to_lowercase() == lower)
        .cloned()
        .collect();
    if !hits.is_empty() {
        return into_match(hits);
    }

    let hits = filter_users(input, users);
    if !hits.is_empty() {
        return into_match(hits);
    }

    UserMatch::None {
        suggestions: suggest(input, users),
    }
}

fn into_match(hits: Vec<UserSummary>) -> UserMatch {
    if hits.len() == 1 {
        UserMatch::Unique(hits.into_iter().next().expect("len == 1"))
    } else {
        UserMatch::Multiple(hits)
    }
}

/// 0 命中时的相近候选：账号或姓名包含输入首字符的用户，最多 5 条。
fn suggest(input: &str, users: &[UserSummary]) -> Vec<UserSummary> {
    let Some(first) = input.chars().next() else {
        return Vec::new();
    };
    let needle = first.to_lowercase().to_string();
    users
        .iter()
        .filter(|u| contains_ci(&u.account, &needle) || u.realname.contains(&needle))
        .take(5)
        .cloned()
        .collect()
}

/// 确认摘要中的解析展示：
///
/// - 输入即账号：`zhanglinan（张李男）`；
/// - 输入是姓名/片段：`李男 → zhanglinan（张李男）`；
/// - 姓名为空时省略括号。
pub fn display_mapping(input: &str, user: &UserSummary) -> String {
    let target = if user.realname.is_empty() {
        user.account.clone()
    } else {
        format!("{}（{}）", user.account, user.realname)
    };
    if input.trim().eq_ignore_ascii_case(&user.account) {
        target
    } else {
        format!("{} → {}", input.trim(), target)
    }
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

    fn sample() -> Vec<UserSummary> {
        vec![
            user("zhanglinan", "张李男"),
            user("liman", "李男"),
            user("wangwu", "王五"),
            user("admin", ""),
        ]
    }

    #[test]
    fn exact_account_match_is_case_insensitive() {
        match match_users("ZhangLinan", &sample()) {
            UserMatch::Unique(u) => assert_eq!(u.account, "zhanglinan"),
            other => panic!("expected Unique, got {other:?}"),
        }
    }

    #[test]
    fn exact_realname_match() {
        match match_users("王五", &sample()) {
            UserMatch::Unique(u) => assert_eq!(u.account, "wangwu"),
            other => panic!("expected Unique, got {other:?}"),
        }
    }

    #[test]
    fn realname_substring_match_hits_unique() {
        // "李男" 同时是 liman 的姓名精确命中——精确规则优先于包含规则。
        match match_users("李男", &sample()) {
            UserMatch::Unique(u) => assert_eq!(u.account, "liman"),
            other => panic!("expected Unique(liman), got {other:?}"),
        }
        // "张李男" 子串命中 zhanglinan 的姓名。
        match match_users("张李", &sample()) {
            UserMatch::Unique(u) => assert_eq!(u.account, "zhanglinan"),
            other => panic!("expected Unique(zhanglinan), got {other:?}"),
        }
    }

    #[test]
    fn substring_match_with_multiple_hits_returns_all_in_input_order() {
        let users = vec![
            user("zhanglinan", "张李男"),
            user("liman", "李男"),
            user("liyan", "李燕"),
        ];
        match match_users("李", &users) {
            UserMatch::Multiple(hits) => {
                let accounts: Vec<&str> = hits.iter().map(|u| u.account.as_str()).collect();
                assert_eq!(accounts, vec!["zhanglinan", "liman", "liyan"]);
            }
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn account_substring_match() {
        match match_users("zhang", &sample()) {
            UserMatch::Unique(u) => assert_eq!(u.account, "zhanglinan"),
            other => panic!("expected Unique, got {other:?}"),
        }
    }

    #[test]
    fn no_match_returns_suggestions_by_first_char() {
        match match_users("李娜娜", &sample()) {
            UserMatch::None { suggestions } => {
                let accounts: Vec<&str> = suggestions.iter().map(|u| u.account.as_str()).collect();
                assert!(accounts.contains(&"zhanglinan"));
                assert!(accounts.contains(&"liman"));
                assert!(suggestions.len() <= 5);
            }
            other => panic!("expected None with suggestions, got {other:?}"),
        }
    }

    #[test]
    fn no_match_without_related_users_has_empty_suggestions() {
        match match_users("赵六", &sample()) {
            UserMatch::None { suggestions } => assert!(suggestions.is_empty()),
            other => panic!("expected None, got {other:?}"),
        }
    }

    #[test]
    fn empty_input_returns_none() {
        match match_users("   ", &sample()) {
            UserMatch::None { suggestions } => assert!(suggestions.is_empty()),
            other => panic!("expected None, got {other:?}"),
        }
    }

    #[test]
    fn empty_realname_never_matches_by_name_rules() {
        // admin 姓名为空：按姓名规则不命中，账号规则仍可用。
        match match_users("admin", &sample()) {
            UserMatch::Unique(u) => assert_eq!(u.account, "admin"),
            other => panic!("expected Unique(admin), got {other:?}"),
        }
    }

    #[test]
    fn filter_users_is_case_insensitive_and_rejects_empty_keyword() {
        let hits = filter_users("WU", &sample());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].account, "wangwu");
        assert!(filter_users("  ", &sample()).is_empty());
    }

    #[test]
    fn display_mapping_shapes() {
        let u = user("zhanglinan", "张李男");
        assert_eq!(display_mapping("李男", &u), "李男 → zhanglinan（张李男）");
        assert_eq!(display_mapping("zhanglinan", &u), "zhanglinan（张李男）");
        assert_eq!(display_mapping("ZhangLinan", &u), "zhanglinan（张李男）");
        let no_name = user("admin", "");
        assert_eq!(display_mapping("admin", &no_name), "admin");
        assert_eq!(display_mapping("管理员", &no_name), "管理员 → admin");
    }
}
