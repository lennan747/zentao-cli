//! 创建体验优化端到端测试：多人指派、创建回执、图片 URL 嵌入。

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
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

fn wrap_locate(url: &str) -> String {
    let inner = serde_json::to_string(&format!(r#"{{"locate":"https://x{url}"}}"#)).unwrap();
    format!(r#"{{"status":"success","data":{}}}"#, inner)
}

async fn mount_users(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/my-task.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("user-list.json")))
        .mount(server)
        .await;
}

#[tokio::test]
async fn create_dry_run_shows_multiple_resolved_assignees() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args([
            "task",
            "create",
            "43",
            "--name",
            "多人任务",
            "--assigned-to",
            "王力,王李男",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("王力 → wangli（王力）"))
        .stdout(predicates::str::contains("王李男 → wanglinan（王李男）"))
        .stdout(predicates::str::contains("[dry-run]"));

    // dry-run 不应发出写请求。
    let requests = server.received_requests().await.unwrap();
    assert!(requests.iter().all(|r| r.method == reqwest::Method::GET));
}

#[tokio::test]
async fn create_yes_prints_receipt_with_link() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    Mock::given(method("POST"))
        .and(path("/task-create-43.json"))
        .and(body_string_contains("assignedTo%5B%5D=wangli"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(wrap_locate("/task-view-999.json")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args([
            "task",
            "create",
            "43",
            "--name",
            "回执任务",
            "--assigned-to",
            "王力",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("999: 回执任务"))
        .stdout(predicates::str::contains("task-view-999.html"));
}

#[tokio::test]
async fn create_json_receipt_is_single_object() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    Mock::given(method("POST"))
        .and(path("/task-create-43.json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(wrap_locate("/task-view-999.json")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    let output = zentao(&home)
        .args([
            "--format",
            "json",
            "task",
            "create",
            "43",
            "--name",
            "json回执",
            "--yes",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let receipt: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(receipt["id"], "999");
    assert_eq!(receipt["title"], "json回执");
    assert!(receipt["url"]
        .as_str()
        .unwrap()
        .ends_with("/task-view-999.html"));
}

#[tokio::test]
async fn create_locate_without_id_reports_null_and_hint() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    Mock::given(method("POST"))
        .and(path("/task-create-43.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(wrap_locate("/my-task.json")))
        .expect(2)
        .mount(&server)
        .await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    // json：id 为 null。
    let output = zentao(&home)
        .args([
            "--format", "json", "task", "create", "43", "--name", "无ID", "--yes",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let receipt: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(receipt["id"].is_null());

    // table：dim 提示。
    zentao(&home)
        .args(["task", "create", "43", "--name", "无ID", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("未能从响应解析新对象 ID"));
}

#[tokio::test]
async fn bug_create_rejects_multiple_assignees() {
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args([
            "bug",
            "create",
            "10",
            "--title",
            "多人Bug",
            "--assigned-to",
            "wangli,wanglinan",
        ])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("仅支持单人"));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn create_image_url_embeds_desc_and_warns_when_unreachable() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let ok_url = format!("{}/ok.png", server.uri());
    let bad_url = format!("{}/bad.png", server.uri());
    Mock::given(method("HEAD"))
        .and(path("/ok.png"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/bad.png"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/task-create-43.json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(wrap_locate("/task-view-999.json")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    // dry-run 摘要里 desc 含两个 <img> 行。
    zentao(&home)
        .args([
            "task",
            "create",
            "43",
            "--name",
            "带图任务",
            "--desc",
            "描述",
            "--image-url",
            &ok_url,
            "--image-url",
            &bad_url,
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "<img src=\"{ok_url}\" />"
        )))
        .stdout(predicates::str::contains(format!(
            "<img src=\"{bad_url}\" />"
        )));

    // 提交时仅对不可达 URL 警告，且仍成功。
    zentao(&home)
        .args([
            "task",
            "create",
            "43",
            "--name",
            "带图任务",
            "--image-url",
            &ok_url,
            "--image-url",
            &bad_url,
            "--yes",
        ])
        .assert()
        .success()
        .stderr(predicates::str::contains("图片 URL 可能不可达"))
        .stderr(predicates::str::contains(&bad_url))
        .stderr(predicates::str::contains(&ok_url).not());
}
