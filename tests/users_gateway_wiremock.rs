//! 用户网关 wiremock 契约测试：users 映射解析、降级链与错误分类。

use std::path::PathBuf;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use zentao_cli::adapters::zentao_v9::{ZentaoV9Client, ZentaoV9UserGateway};
use zentao_cli::application::UserGateway;
use zentao_cli::domain::QueryError;

fn fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e))
}

async fn mount(server: &MockServer, path_expr: &str, body: String) {
    Mock::given(method("GET"))
        .and(path(path_expr))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(server)
        .await;
}

async fn gateway(server: &MockServer) -> ZentaoV9UserGateway {
    let client = ZentaoV9Client::new(server.uri()).unwrap();
    client.import_cookies("abc123").unwrap();
    ZentaoV9UserGateway::new(client)
}

#[tokio::test]
async fn list_users_parses_primary_source() {
    let server = MockServer::start().await;
    mount(&server, "/my-task.json", fixture("user-list.json")).await;

    let users = gateway(&server).await.list_users().await.unwrap();
    let accounts: Vec<&str> = users.iter().map(|u| u.account.as_str()).collect();
    assert_eq!(
        accounts,
        vec!["admin", "demo-user", "wangli", "wanglinan", "zhangsan"]
    );
    let wanglinan = users.iter().find(|u| u.account == "wanglinan").unwrap();
    assert_eq!(wanglinan.realname, "王李男");

    // 主源成功后不应触碰备源。
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn list_users_falls_back_when_primary_lacks_users() {
    let server = MockServer::start().await;
    // task-list.json 是真实的 my-task 响应形态，无 users 字段。
    mount(&server, "/my-task.json", fixture("task-list.json")).await;
    mount(&server, "/project-all-0.json", fixture("user-list.json")).await;

    let users = gateway(&server).await.list_users().await.unwrap();
    assert_eq!(users.len(), 5);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn list_users_parses_array_form() {
    let server = MockServer::start().await;
    mount(&server, "/my-task.json", fixture("user-list-array.json")).await;

    let users = gateway(&server).await.list_users().await.unwrap();
    assert_eq!(users.len(), 5);
    assert!(users
        .iter()
        .any(|u| u.account == "zhangsan" && u.realname == "张三"));
}

#[tokio::test]
async fn list_users_returns_forbidden_when_all_sources_denied() {
    let server = MockServer::start().await;
    let deny =
        r#"{"status":"success","data":"{\"locate\":\"https://x/user-deny-user-index.json\"}"}"#;
    mount(&server, "/my-task.json", deny.to_string()).await;
    mount(&server, "/project-all-0.json", deny.to_string()).await;

    let err = gateway(&server).await.list_users().await.unwrap_err();
    assert!(matches!(err, QueryError::Forbidden));
}

#[tokio::test]
async fn list_users_propagates_session_expired_without_trying_backup() {
    let server = MockServer::start().await;
    mount(&server, "/my-task.json", fixture("session-expired.json")).await;
    // 备源不挂载：若被触碰会得到 404 → Remote，而非 SessionExpired。

    let err = gateway(&server).await.list_users().await.unwrap_err();
    assert!(matches!(err, QueryError::SessionExpired));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn list_users_reports_last_error_when_all_sources_fail() {
    let server = MockServer::start().await;
    // 两个来源都 404（未挂载）→ Remote（HTTP 错误）。
    let err = gateway(&server).await.list_users().await.unwrap_err();
    assert!(matches!(err, QueryError::Remote(_)));
}
