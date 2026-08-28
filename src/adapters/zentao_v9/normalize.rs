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

/// 解码富文本中常见的 HTML 实体（用于 <img> src 属性值）。
fn decode_attr_entities(s: &str) -> String {
    let mut out = s.to_string();
    for (entity, decoded) in [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
    ] {
        out = out.replace(entity, decoded);
    }
    out
}

/// 从富文本 HTML 中提取 `<img src="...">` 的 src 列表（去重、保序）。
///
/// 不区分大小写；容忍属性顺序与单/双引号；属性值中的常见 HTML 实体会解码。
pub fn extract_image_urls(input: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let raw = input.as_bytes();
    let mut i = 0usize;
    while i + 4 < raw.len() {
        // 找到 "<img"（大小写不敏感；排除 <image 之类的前缀误匹配）
        if raw[i] == b'<'
            && raw[i + 1..i + 4].eq_ignore_ascii_case(b"img")
            && (raw[i + 4].is_ascii_whitespace() || raw[i + 4] == b'/' || raw[i + 4] == b'>')
        {
            // 在该标签内寻找 src=
            let mut j = i + 4;
            while j < raw.len() && raw[j] != b'>' {
                if j + 4 <= raw.len() && raw[j..j + 4].eq_ignore_ascii_case(b"src=") {
                    let mut k = j + 4;
                    let quote = if k < raw.len() && (raw[k] == b'"' || raw[k] == b'\'') {
                        let q = raw[k];
                        k += 1;
                        Some(q)
                    } else {
                        None
                    };
                    let start = k;
                    let end = match quote {
                        Some(q) => {
                            while k < raw.len() && raw[k] != q {
                                k += 1;
                            }
                            k
                        }
                        None => {
                            while k < raw.len() && !raw[k].is_ascii_whitespace() && raw[k] != b'>' {
                                k += 1;
                            }
                            // 无引号属性值末尾的 `/` 可能是自闭合标签标记（如 src=a.png/>）
                            if k < raw.len() && raw[k] == b'>' && k > start && raw[k - 1] == b'/' {
                                k -= 1;
                            }
                            k
                        }
                    };
                    let value = decode_attr_entities(&String::from_utf8_lossy(&raw[start..end]))
                        .trim()
                        .to_string();
                    if !value.is_empty() && !urls.contains(&value) {
                        urls.push(value);
                    }
                    j = end;
                } else {
                    j += 1;
                }
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    urls
}

/// 把 <img> src 解析为可访问的绝对 URL。
///
/// - `http://`/`https://`/`data:` 原样返回；
/// - `//host/...` 用 server 的协议补全；
/// - `/path` 或相对路径拼接到 server（末尾 `/` 已按约定去掉）。
pub fn resolve_image_url(src: &str, server: &str) -> String {
    let src = src.trim();
    if src.is_empty() {
        return String::new();
    }
    if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("data:") {
        return src.to_string();
    }
    let server = server.trim_end_matches('/');
    if let Some(rest) = src.strip_prefix("//") {
        let scheme = server.split("://").next().unwrap_or("http");
        return format!("{scheme}://{rest}");
    }
    if src.starts_with('/') {
        format!("{server}{src}")
    } else {
        format!("{server}/{src}")
    }
}

/// 从富文本 HTML 中提取并解析为绝对 URL 的图片列表。
pub fn resolve_image_urls(input: &str, server: &str) -> Vec<String> {
    extract_image_urls(input)
        .iter()
        .map(|s| resolve_image_url(s, server))
        .filter(|s| !s.is_empty())
        .collect()
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

    #[test]
    fn extract_image_urls_finds_img_src() {
        let html = r#"<p>步骤1</p><p><img src="data/upload/2026/08/abc.png" alt="" /></p>"#;
        assert_eq!(
            extract_image_urls(html),
            vec!["data/upload/2026/08/abc.png"]
        );
    }

    #[test]
    fn extract_image_urls_handles_case_quotes_and_order() {
        let html = concat!(
            r#"<IMG SRC="a.png">"#,
            r#"<img alt="x" src='b.png' class="c">"#,
            r#"<img src=c.png/>"#,
        );
        assert_eq!(extract_image_urls(html), vec!["a.png", "b.png", "c.png"]);
    }

    #[test]
    fn extract_image_urls_dedupes_and_decodes_entities() {
        let html = r#"<img src="a.png?x=1&amp;y=2"><img src="a.png?x=1&amp;y=2"><img src="&quot;b.png&quot;">"#;
        // &quot; 引号解码后仍是 b.png
        assert_eq!(extract_image_urls(html), vec!["a.png?x=1&y=2", "\"b.png\""]);
    }

    #[test]
    fn extract_image_urls_ignores_non_img_prefix() {
        assert!(extract_image_urls("<image src=\"x.png\"><p><imgs src=\"y.png\">").is_empty());
    }

    #[test]
    fn resolve_image_url_absolute_and_relative() {
        let server = "https://zentao.example.com";
        assert_eq!(
            resolve_image_url("https://cdn.x.com/a.png", server),
            "https://cdn.x.com/a.png"
        );
        assert_eq!(
            resolve_image_url("data:image/png;base64,AAA", server),
            "data:image/png;base64,AAA"
        );
        assert_eq!(
            resolve_image_url("//cdn.x.com/a.png", server),
            "https://cdn.x.com/a.png"
        );
        assert_eq!(
            resolve_image_url("/data/upload/a.png", server),
            "https://zentao.example.com/data/upload/a.png"
        );
        assert_eq!(
            resolve_image_url("data/upload/a.png", server),
            "https://zentao.example.com/data/upload/a.png"
        );
        // server 末尾带 / 也不重复
        assert_eq!(
            resolve_image_url("/a.png", "https://x.com/"),
            "https://x.com/a.png"
        );
    }
}
