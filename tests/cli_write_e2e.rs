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

// ---- 指派人解析（姓名 → 账号）----

/// 挂载用户列表来源（my-task.json 顶层 users 映射，虚构账号）。
async fn mount_users(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/my-task.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("user-list.json")))
        .mount(server)
        .await;
}

#[tokio::test]
async fn assign_by_realname_resolves_unique_and_dry_run_issues_no_write() {
    // AC1 + AC8：只挂用户列表来源；task-view/写路由不挂载，若被触碰会 404 导致失败。
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "assign", "946", "李男", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("李男 → wanglinan（王李男）"))
        .stdout(predicates::str::contains("[dry-run]"));

    let requests = server.received_requests().await.unwrap();
    assert!(requests.iter().all(|r| r.method == reqwest::Method::GET));
    assert_eq!(requests.len(), 1);
}

#[tokio::test]
async fn assign_multiple_candidates_non_tty_errors_with_candidate_list() {
    // AC2：非 TTY（assert_cmd 天然无 TTY）多候选 → 退出码 6 + stderr 列全部候选。
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "assign", "946", "王"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("找到多个匹配"))
        .stderr(predicates::str::contains("wangli（王力）"))
        .stderr(predicates::str::contains("wanglinan（王李男）"))
        .stderr(predicates::str::contains("账号精确指定"));
}

#[tokio::test]
async fn assign_no_match_errors_with_suggestions() {
    // AC4：0 命中 → 退出码 6 + 相近候选建议 + user search 指引。
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "assign", "946", "李娜娜"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("未找到用户 \"李娜娜\""))
        .stderr(predicates::str::contains("wanglinan（王李男）"))
        .stderr(predicates::str::contains("user search"));
}

#[tokio::test]
async fn assign_by_exact_account_keeps_working() {
    // AC6：账号精确输入不回归；摘要显示 账号（姓名）。
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "assign", "946", "demo-user", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("demo-user（演示用户）"));
}

#[tokio::test]
async fn assign_empty_account_rejected_before_network() {
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "assign", "946", "   ", "--dry-run"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("不能为空"));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn user_list_unavailable_ascii_input_passes_through_with_warning() {
    // AC7a：用户列表来源全部 404 → ASCII 输入按账号直通（保持旧行为）+ dim 警告。
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "assign", "946", "someone", "--dry-run"])
        .assert()
        .success()
        .stderr(predicates::str::contains("无法获取用户列表"))
        .stdout(predicates::str::contains("someone"));
}

#[tokio::test]
async fn user_list_unavailable_non_ascii_input_errors() {
    // AC7b：用户列表不可用时姓名（非 ASCII）输入报错，退出码 6。
    let server = MockServer::start().await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["task", "assign", "946", "李男"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("无法获取用户列表"))
        .stderr(predicates::str::contains("账号精确指派"));
}

#[tokio::test]
async fn create_with_assigned_to_name_resolves_in_summary() {
    // 可选参数入口：--assigned-to 姓名精确命中，摘要显示映射。
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
            "测试任务",
            "--assigned-to",
            "王力",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("王力 → wangli（王力）"));
}

#[tokio::test]
async fn bug_assign_by_name_resolves() {
    // bug 侧位置参数入口同构生效。
    let server = MockServer::start().await;
    mount_users(&server).await;
    let home = TempDir::new().unwrap();
    seed_session(&home, &server.uri());

    zentao(&home)
        .args(["bug", "assign", "41292", "李男", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("李男 → wanglinan（王李男）"));
}
