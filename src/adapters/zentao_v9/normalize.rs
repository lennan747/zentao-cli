//! 旧版响应字段到领域 DTO 的通用归一化辅助。

use serde::de::DeserializeOwned;
use serde_json::Value;

/// 读取字符串字段，缺失时返回空字符串。
pub fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

/// 读取可选字符串字段；空串视为 None。
pub fn opt_str(v: &Value, key: &str) -> Option<String> {
    let s = str_field(v, key);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// 读取日期字段。禅道旧版用 `0000-00-00` 表示空日期。
pub fn opt_date(v: &Value, key: &str) -> Option<String> {
    let s = str_field(v, key);
    if s.is_empty() || s.starts_with("0000-00-00") {
        None
    } else {
        Some(s)
    }
}

/// 读取数字字段（旧版把数字也存成字符串）。
pub fn num_field<T: std::str::FromStr + Default>(v: &Value, key: &str) -> T {
    str_field(v, key).parse().unwrap_or_default()
}

/// 解析枚举字段；`#[serde(other)]` 保证未知值落入 Unknown。
pub fn enum_field<T: DeserializeOwned + Default>(v: &Value, key: &str) -> T {
    let raw = str_field(v, key);
    serde_json::from_value(Value::String(raw)).unwrap_or_default()
}

/// 去除 HTML 标签并解码常见实体，用于描述/重现步骤等富文本字段。
pub fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    for (entity, decoded) in [
        ("&nbsp;", " "),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
    ] {
        out = out.replace(entity, decoded);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_tags_and_entities() {
        let v = json!({"desc": "<p>hello&nbsp;&amp;&nbsp;world</p>"});
        assert_eq!(strip_html(&str_field(&v, "desc")), "hello & world");
    }

    #[test]
    fn zero_dates_are_none() {
        let v = json!({"a": "0000-00-00 00:00:00", "b": "2026-06-11", "c": ""});
        assert_eq!(opt_date(&v, "a"), None);
        assert_eq!(opt_date(&v, "b"), Some("2026-06-11".into()));
        assert_eq!(opt_date(&v, "c"), None);
        assert_eq!(opt_date(&v, "missing"), None);
    }

    #[test]
    fn numeric_strings_parse() {
        let v = json!({"n": "42", "bad": "x"});
        assert_eq!(num_field::<u64>(&v, "n"), 42);
        assert_eq!(num_field::<u64>(&v, "bad"), 0);
    }
}
