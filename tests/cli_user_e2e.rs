//! user 子命令端到端测试：list/search 的 table/json 输出与参数校验。

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn zentao(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("zentao-cli").unwrap();
    cmd.env("ZENTAO_CLI_HOME", home.path());
    cmd
}

/// 直接写会话文件，跳过登录流程。
fn seed_session(home: &TempDir, server_uri: &str) {
    let path: PathBuf = home.path().join("session-default.json");
    let content = serde_json::json!({
        "server": server_uri,
        "cookie": "abc123",
    })
    .to_string();
    std::fs::write(path, content).unwrap();
}

fn fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e))
}

async fn mount_users(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/my-task.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("user-list.json")))
        .mount(server)
        .await;
}

#[tokio::test]
async fn user_list_prints_account_realname_table() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["user", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("账号"))
        .stdout(predicates::str::contains("姓名"))
        .stdout(predicates::str::contains("wanglinan"))
        .stdout(predicates::str::contains("王李男"))
        .stdout(predicates::str::contains("Total: 5"));
}

#[tokio::test]
async fn user_list_json_is_valid_array() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    let output = zentao(&home)
        .args(["--format", "json", "user", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let users: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert_eq!(users.len(), 5);
    assert_eq!(users[0]["account"], "admin");
    assert_eq!(users[0]["realname"], "管理员");
}

#[tokio::test]
async fn user_search_filters_by_account_or_realname() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["user", "search", "王"])
        .assert()
        .success()
        .stdout(predicates::str::contains("wangli"))
        .stdout(predicates::str::contains("wanglinan"))
        .stdout(predicates::str::contains("zhangsan").not())
        .stdout(predicates::str::contains("Total: 2"));
}

#[tokio::test]
async fn user_search_without_hit_prints_empty() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["user", "search", "钱七"])
        .assert()
        .success()
        .stdout(predicates::str::contains("（无数据）"));
}

#[tokio::test]
async fn user_search_empty_keyword_rejected_before_network() {
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["user", "search", "   "])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("搜索关键词不能为空"));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}
