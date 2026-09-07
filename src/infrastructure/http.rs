//! 通用 HTTP 探测（不携带禅道会话语义）。

use std::time::Duration;

/// best-effort 可达性探测：HEAD 请求，2xx/3xx 视为可达；任何错误或非 2xx/3xx 视为不可达。
///
/// 仅用于提交前警告（如外部图片 URL），不阻断流程。
pub async fn url_reachable(url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    else {
        return false;
    };
    match client.head(url).send().await {
        Ok(response) => {
            let status = response.status();
            status.is_success() || status.is_redirection()
        }
        Err(_) => false,
    }
}
