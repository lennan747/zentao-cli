//! CLI 写命令端到端测试：确认拦截、--dry-run/--yes、参数校验与提交。

use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const RELOAD_BODY: &str = "<html><meta charset='utf-8'/><style>body{background:white}</style><script>if(parent !== window) parent.location.reload(true);\n</script>";

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

/// 挂载评论 POST mock；若 CLI 意外发请求会得到 404，从而让断言失败。
async fn mount_comment_mock(server: &MockServer, task_id: &str) {
    Mock::given(method("POST"))
        .and(path(format!("/action-comment-task-{task_id}.html")))
        .and(body_string_contains("comment="))
        .respond_with(ResponseTemplate::new(200).set_body_string(RELOAD_BODY))
        .mount(server)
        .await;
}

#[tokio::test]
async fn dry_run_prints_summary_and_issues_no_request() {
    let server = MockServer::start().await;
    mount_comment_mock(&server, "946").await;

    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args([
            "task",
            "comment",
            "946",
            "--comment",
            "测试评论",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("评论任务 946"))
        .stdout(predicates::str::contains("[dry-run]"))
        .stdout(predicates::str::contains("测试评论"));
}

#[tokio::test]
async fn without_tty_and_flags_write_is_refused() {
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "comment", "946", "--comment", "测试评论"])
        .assert()
        .code(7)
        .stderr(predicates::str::contains("需要交互确认"));
}

#[tokio::test]
async fn yes_flag_executes_write() {
    let server = MockServer::start().await;
    mount_comment_mock(&server, "946").await;

    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "comment", "946", "--comment", "测试评论", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("已提交成功"));
}

#[tokio::test]
async fn edit_without_fields_is_rejected_before_network() {
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "edit", "946", "--yes"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("未提供任何要修改的字段"));
}

#[tokio::test]
async fn empty_comment_is_rejected_before_network() {
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "comment", "946", "--comment", "", "--yes"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("评论内容不能为空"));
}

#[tokio::test]
async fn denied_permission_reports_forbidden_exit_code() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/task-view-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("task-detail-done.json")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/task-close-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"status":"success","data":"{\"locate\":\"https://x/user-deny-task-close.json\"}"}"#,
        ))
        .mount(&server)
        .await;

    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "close", "946", "--yes"])
        .assert()
        .code(4)
        .stderr(predicates::str::contains("无权限"));
}

#[tokio::test]
async fn start_refuses_zero_left_before_network() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/task-view-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("task-detail.json")))
        .mount(&server)
        .await;

    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "start", "946", "--yes"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("left（剩余工时）为 0"));
}

#[tokio::test]
async fn finish_requires_consumed_flag() {
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "finish", "946", "--yes"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("consumed"));
}

#[tokio::test]
async fn finish_posts_consumed_baseline_and_current() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/task-view-978.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("task-detail.json")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/task-finish-978.json"))
        .and(body_string_contains("currentConsumed=2"))
        .and(body_string_contains("consumed=0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"status":"success","data":"{\"locate\":\"https://x/task-view-978.json\"}"}"#,
        ))
        .mount(&server)
        .await;

    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "finish", "978", "--consumed", "2", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("已提交成功"));
}
