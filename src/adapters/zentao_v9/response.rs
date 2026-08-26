use serde::Deserialize;
use serde_json::Value;

use crate::domain::QueryError;

/// 禅道旧版控制器返回的最外层 JSON 包络。
#[derive(Debug, Clone, Deserialize)]
struct Envelope {
    status: String,
    data: String,
}

/// 解析远端响应体，返回 `data` 字段对应的 JSON `Value`。
///
/// 旧版禅道 `.json` 接口的响应通常被 `<html><body>` 包裹，内部是一个或多个连续的
/// JSON 包络对象：`{"status":"success","data":"..."}`，其中 `data` 本身又是 JSON 字符串。
/// 错误时 `data` 可能包含 `result:fail`、`message` 或 `locate` 等字段。
pub fn parse_body(body: &str) -> Result<Value, QueryError> {
    let json_text = extract_json_segments(body);

    // 旧版控制器可能在同一响应里连续输出多个 JSON 包络对象。
    // 每个包络格式为 {"status":"success","data":"..."}，data 本身又是 JSON 字符串。
    // 多个包络的 data 字段需要合并，例如第一个给出 result:fail+message，第二个给出 locate:back。
    let segments = split_json_objects(&json_text);
    if segments.is_empty() {
        return Err(QueryError::ParseError("empty response".into()));
    }

    let mut merged_data: Option<Value> = None;
    let mut any_failure = false;

    for segment in segments {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let envelope: Envelope = serde_json::from_str(segment)
            .map_err(|e| QueryError::ParseError(format!("invalid envelope segment: {e}")))?;

        if envelope.status != "success" {
            any_failure = true;
        }

        let data: Value = serde_json::from_str(&envelope.data)
            .map_err(|e| QueryError::ParseError(format!("data field is not JSON: {e}")))?;
        merged_data = Some(merge_data(merged_data.take(), data));
    }

    let data = merged_data.ok_or_else(|| QueryError::ParseError("empty response".into()))?;

    // 即使所有 status 都是 success，data 内部仍可能携带业务错误。
    if any_failure {
        classify_data(&data)?;
        return Ok(data);
    }

    classify_data(&data)?;
    Ok(data)
}

/// 从可能包含 `<html><body>` 的文本中提取 JSON 片段。
fn extract_json_segments(body: &str) -> String {
    let body = body.trim();

    // 直接就是 JSON 的情况（测试/mock 常见）。
    if body.starts_with('{') || body.starts_with('[') {
        return body.to_string();
    }

    // 旧版响应：<html><body>{...}</body></html>
    let start = body.find('{').unwrap_or(0);
    let end = body.rfind('}').map(|i| i + 1).unwrap_or(body.len());
    body[start..end].to_string()
}

/// 将可能连续拼接的 JSON 对象拆分为独立字符串。
fn split_json_objects(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;

    for (i, ch) in text.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    result.push(&text[start..=i]);
                }
            }
            _ => {}
        }
    }

    if result.is_empty() {
        result.push(text);
    }

    result
}

fn merge_data(a: Option<Value>, b: Value) -> Value {
    match (a, b) {
        (Some(Value::Object(mut a_map)), Value::Object(b_map)) => {
            for (k, v) in b_map {
                a_map.insert(k, v);
            }
            Value::Object(a_map)
        }
        (_, b) => b,
    }
}

fn classify_data(data: &Value) -> Result<(), QueryError> {
    if let Some(obj) = data.as_object() {
        if obj.get("result").and_then(|v| v.as_str()) == Some("fail") {
            let msg = message_text(obj);
            if msg.contains("并不存在") || msg.contains("不存在") {
                return Err(QueryError::NotFound);
            }
            return Err(QueryError::Rejected(msg));
        }

        if let Some(locate) = obj.get("locate").and_then(|v| v.as_str()) {
            // 无权限：旧版跳转到 user-deny-{module}-{method} 拒绝页。
            if locate.contains("user-deny") {
                return Err(QueryError::Forbidden);
            }
            if locate.contains("login") || locate.contains("user-login") {
                return Err(QueryError::SessionExpired);
            }
            // locate == "back" 等通常表示错误返回上一页。
            if obj.get("message").is_some() {
                let msg = message_text(obj);
                if msg.contains("权限") || msg.contains("无权") {
                    return Err(QueryError::Forbidden);
                }
            }
        }

        if obj.get("message").is_some() {
            let msg = message_text(obj);
            if msg.contains("权限") || msg.contains("无权") {
                return Err(QueryError::Forbidden);
            }
        }
    }

    Ok(())
}

/// 提取 message：字符串、数组或对象（字段→消息）统一拼为文本。
fn message_text(obj: &serde_json::Map<String, Value>) -> String {
    match obj.get("message") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => join_messages(items.iter()),
        Some(Value::Object(map)) => join_messages(map.values()),
        _ => "对象不存在或无权访问".to_string(),
    }
}

fn join_messages<'a>(values: impl Iterator<Item = &'a Value>) -> String {
    values
        .filter_map(|v| match v {
            Value::String(s) => Some(decode_message_string(s)),
            Value::Array(items) => {
                let parts: Vec<String> = items
                    .iter()
                    .filter_map(|x| x.as_str().map(decode_message_string))
                    .collect();
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join("; "))
                }
            }
            other => Some(other.to_string()),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// message 里的双引号是包络转义遗留，还原成真实文本。
fn decode_message_string(s: &str) -> String {
    serde_json::from_str::<String>(&format!("\"{s}\""))
        .unwrap_or_else(|_| s.to_string())
}

/// 解析“JS 风格”写响应（评论等接口不返回包络，而是父窗刷新脚本）。
///
/// 成功：`<script>if(parent !== window) ...reload...`（无 alert）。
/// 失败：脚本内先出现 `alert('消息')`。
pub fn parse_alert_response(body: &str) -> Result<(), QueryError> {
    let Some(start) = body.find("alert(") else {
        return Ok(());
    };
    let rest = &body[start + "alert(".len()..];
    let inner = rest
        .trim_start_matches('\'')
        .trim_start_matches('"')
        .to_string();
    // 找到结束引号（消息内可能出现转义引号，取最后一个 ')'）。
    let inner = match inner.find("')") {
        Some(i) => inner[..i].to_string(),
        None => inner
            .find("\")")
            .map(|i| inner[..i].to_string())
            .unwrap_or(inner),
    };
    let msg = inner
        .replace("\\'", "'")
        .replace("\\\"", "\"")
        .replace("\\n", "\n")
        .trim_end()
        .trim()
        .to_string();
    if msg.is_empty() {
        return Ok(());
    }
    Err(QueryError::Rejected(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_html_wrapped_success() {
        let body = r#"<html><body>{"status":"success","data":"{\"id\":\"1\",\"name\":\"p\"}","md5":"abc"}</body></html>"#;
        let value = parse_body(body).unwrap();
        assert_eq!(value["id"], "1");
        assert_eq!(value["name"], "p");
    }

    #[test]
    fn parses_multiple_json_segments() {
        let body = r#"{"status":"success","data":"{\"result\":\"fail\",\"message\":\"抱歉，您访问的对象并不存在！\"}"}{"status":"success","data":"{\"locate\":\"back\"}"}"#;
        let err = parse_body(body).unwrap_err();
        assert!(matches!(err, QueryError::NotFound));
    }

    #[test]
    fn detects_session_expired() {
        let body = r#"{"status":"success","data":"{\"locate\":\"\/user-login.html\"}"}"#;
        let err = parse_body(body).unwrap_err();
        assert!(matches!(err, QueryError::SessionExpired));
    }

    #[test]
    fn detects_forbidden() {
        let body = r#"{"status":"success","data":"{\"message\":\"您无权访问该项目！\"}"}"#;
        let err = parse_body(body).unwrap_err();
        assert!(matches!(err, QueryError::Forbidden));
    }

    #[test]
    fn detects_user_deny_redirect() {
        let body = r#"{"status":"success","data":"{\"locate\":\"https://x/user-deny-task-close.json\"}"}"#;
        let err = parse_body(body).unwrap_err();
        assert!(matches!(err, QueryError::Forbidden));
    }

    #[test]
    fn rejects_with_array_validation_messages() {
        let body = r#"{"status":"success","data":"{\"result\":\"fail\",\"message\":[\"\\\"本次消耗\\\"不能为0\"]}"}"#;
        let err = parse_body(body).unwrap_err();
        match err {
            QueryError::Rejected(msg) => assert!(msg.contains("本次消耗")),
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[test]
    fn write_success_with_locate_passes() {
        let body = r#"{"status":"success","data":"{\"locate\":\"https://x/task-view-946.json\"}"}"#;
        assert!(parse_body(body).is_ok());
    }

    #[test]
    fn alert_script_without_alert_is_success() {
        let body = "<html><meta charset='utf-8'/><style>body{background:white}</style><script>if(parent !== window) parent.location.reload(true);\n</script>";
        assert!(parse_alert_response(body).is_ok());
    }

    #[test]
    fn alert_script_with_alert_is_rejected() {
        let body = "<html><script>alert('\"本次消耗\"不能为0\\n')\n</script>";
        let err = parse_alert_response(body).unwrap_err();
        match err {
            QueryError::Rejected(msg) => assert!(msg.contains("本次消耗")),
            other => panic!("expected Rejected, got {other:?}"),
        }
    }
}
