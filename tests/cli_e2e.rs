//! CLI 端到端测试：真实二进制 + wiremock 模拟禅道站点 + 隔离配置目录。

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e))
}

fn zentao(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("zentao-cli").unwrap();
    cmd.env("ZENTAO_CLI_HOME", home.path());
    cmd
}

async fn mount_login_flow(server: &MockServer, probe_body: String) {
    Mock::given(method("GET"))
        .and(path("/user-login-Lw==.html"))
        .respond_with(ResponseTemplate::new(200))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user-refreshRandom.html"))
        .respond_with(ResponseTemplate::new(200).set_body_string("596986874"))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/user-login.html"))
        .and(body_string_contains("account=example-user"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("set-cookie", "zentaosid=abc123; path=/")
                .set_body_string("<html><body>ok</body></html>"),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/my-task.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(probe_body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn login_then_query_reuses_session_across_processes() {
    let server = MockServer::start().await;
    mount_login_flow(&server, fixture("task-list.json")).await;
    Mock::given(method("GET"))
        .and(path("/project-index.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("project-list.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/project-view-43.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("project-detail.json")))
        .mount(&server)
        .await;

    let home = TempDir::new().unwrap();

    // 登录：密码走 stdin（测试环境无 TTY）。
    zentao(&home)
        .args([
            "login",
            "--server",
            &server.uri(),
            "--account",
            "example-user",
        ])
        .write_stdin("secret\n")
        .assert()
        .success();

    // 会话文件权限必须受限（SEC-002）。
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let session_path: PathBuf = home.path().join("session-default.json");
        let mode = std::fs::metadata(&session_path)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "session file must be 0600");
        let content = std::fs::read_to_string(&session_path).unwrap();
        assert!(
            !content.contains("secret"),
            "session must not contain password"
        );
    }

    // 新进程复用会话，无需再次输入密码（AUTH-003）。
    zentao(&home)
        .args(["project", "list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("示例项目-教育"));

    // JSON 输出必须是合法 JSON（FORMAT-001）。
    let output = zentao(&home)
        .args(["project", "get", "43", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert_eq!(parsed["name"], "示例项目-教育");
    assert_eq!(parsed["code"], "YDJY");
}

#[tokio::test]
async fn task_and_bug_commands_work() {
    let server = MockServer::start().await;
    mount_login_flow(&server, fixture("task-list.json")).await;
    Mock::given(method("GET"))
        .and(path("/task-view-947.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("task-detail.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/my-bug-assignedTo.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("bug-list.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/bug-view-41292.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("bug-detail.json")))
        .mount(&server)
        .await;

    let home = TempDir::new().unwrap();
    zentao(&home)
        .args([
            "login",
            "--server",
            &server.uri(),
            "--account",
            "example-user",
        ])
        .write_stdin("secret\n")
        .assert()
        .success();

    zentao(&home)
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("示例任务-退款处理"))
        .stdout(predicates::str::contains("─"))
        .stdout(predicates::str::contains("优先级"))
        .stdout(predicates::str::contains("指派给"))
        .stdout(predicates::str::contains("● 进行中").or(predicates::str::contains("○ 等待中")));

    zentao(&home)
        .args(["task", "get", "947"])
        .assert()
        .success()
        .stdout(predicates::str::contains("示例任务-退款处理"))
        .stdout(predicates::str::contains("字段"))
        .stdout(predicates::str::contains("○ 等待中"));

    zentao(&home)
        .args(["bug", "list", "--assigned-to", "me"])
        .assert()
        .success()
        .stdout(predicates::str::contains("示例Bug-界面调整"));

    zentao(&home)
        .args(["bug", "get", "41292", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("示例产品"));
}

#[tokio::test]
async fn bug_get_shows_steps_images() {
    let server = MockServer::start().await;
    mount_login_flow(&server, fixture("task-list.json")).await;
    let data = r#"{"status":"success","data":"{\"title\":\"t\",\"productName\":\"示例产品\",\"bug\":{\"id\":\"41824\",\"title\":\"带图Bug\",\"status\":\"active\",\"severity\":\"3\",\"pri\":\"3\",\"steps\":\"<p>步骤1</p><p><img src=\\\"data/upload/a.png\\\"></p>\"}}"}"#.to_string();
    Mock::given(method("GET"))
        .and(path("/bug-view-41824.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(data))
        .mount(&server)
        .await;

    let home = TempDir::new().unwrap();
    zentao(&home)
        .args([
            "login",
            "--server",
            &server.uri(),
            "--account",
            "example-user",
        ])
        .write_stdin("secret\n")
        .assert()
        .success();

    let expected_url = format!("{}/data/upload/a.png", server.uri());

    // JSON 模式：steps_images 绝对 URL 数组
    zentao(&home)
        .args(["bug", "get", "41824", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("steps_images"))
        .stdout(predicates::str::contains(&expected_url));

    // table 模式：图片 URL 并入「重现步骤」单元格，不再有独立「重现步骤图片」行
    zentao(&home)
        .args(["bug", "get", "41824"])
        .assert()
        .success()
        .stdout(predicates::str::contains("重现步骤"))
        .stdout(predicates::str::contains(&expected_url))
        .stdout(predicates::str::contains("重现步骤图片").not());
}

#[tokio::test]
async fn logout_clears_session_and_next_query_requires_login() {
    let server = MockServer::start().await;
    mount_login_flow(&server, fixture("task-list.json")).await;

    let home = TempDir::new().unwrap();
    zentao(&home)
        .args([
            "login",
            "--server",
            &server.uri(),
            "--account",
            "example-user",
        ])
        .write_stdin("secret\n")
        .assert()
        .success();

    zentao(&home)
        .args(["logout"])
        .assert()
        .success()
        .stdout(predicates::str::contains("已退出登录"));

    // 未登录查询返回认证错误退出码 3。
    zentao(&home)
        .args(["task", "list"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicates::str::contains("登录"));
}

#[tokio::test]
async fn wrong_password_fails_without_saving_session() {
    let server = MockServer::start().await;
    // 登录 POST 成功返回 200，但探测端点返回会话失效 → 凭据错误。
    mount_login_flow(&server, fixture("session-expired.json")).await;

    let home = TempDir::new().unwrap();
    zentao(&home)
        .args([
            "login",
            "--server",
            &server.uri(),
            "--account",
            "example-user",
        ])
        .write_stdin("wrong-password\n")
        .assert()
        .failure()
        .code(3)
        .stderr(predicates::str::contains("账号或密码错误"));

    assert!(
        !home.path().join("session-default.json").exists(),
        "failed login must not save session"
    );
}

#[tokio::test]
async fn empty_list_prints_hint() {
    let server = MockServer::start().await;
    mount_login_flow(&server, fixture("task-list.json")).await;
    Mock::given(method("GET"))
        .and(path("/project-index.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"status":"success","data":"{\"title\":\"项目主页\",\"projects\":{}}"}"#,
        ))
        .mount(&server)
        .await;

    let home = TempDir::new().unwrap();
    zentao(&home)
        .args([
            "login",
            "--server",
            &server.uri(),
            "--account",
            "example-user",
        ])
        .write_stdin("secret\n")
        .assert()
        .success();

    zentao(&home)
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("（无数据）"));
}

#[tokio::test]
async fn long_task_name_is_truncated() {
    let server = MockServer::start().await;
    let long_name = "超长任务名称".repeat(20);
    let inner = serde_json::json!({
        "tasks": [{
            "id": "1",
            "project": "43",
            "projectName": "示例项目",
            "name": long_name,
            "status": "wait",
            "pri": "2",
            "assignedTo": "example-user",
        }],
        "pager": {"recTotal": 1, "recPerPage": 20, "pageTotal": 1, "pageID": 1},
    });
    let body = serde_json::json!({
        "status": "success",
        "data": inner.to_string(),
    })
    .to_string();
    mount_login_flow(&server, body).await;

    let home = TempDir::new().unwrap();
    zentao(&home)
        .args([
            "login",
            "--server",
            &server.uri(),
            "--account",
            "example-user",
        ])
        .write_stdin("secret\n")
        .assert()
        .success();

    zentao(&home)
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("..."));
}
