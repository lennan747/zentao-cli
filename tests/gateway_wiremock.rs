//! 网关层 wiremock 集成测试：使用脱敏契约夹具验证登录与查询。

use std::path::PathBuf;
use std::time::Duration;

use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockGuard, MockServer, ResponseTemplate};

use zentao_cli::adapters::zentao_v9::{
    ZentaoV9AuthGateway, ZentaoV9BugGateway, ZentaoV9Client, ZentaoV9ProjectGateway,
    ZentaoV9TaskGateway,
};
use zentao_cli::application::{
    AuthGateway, BugGateway, BugQuery, Credentials, ProjectGateway, ProjectQuery, Session,
    TaskGateway, TaskQuery,
};
use zentao_cli::domain::{AuthError, EntityId, QueryError};

fn fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e))
}

/// 将 data JSON 包装为旧版禅道响应包络。
fn wrap(data_json: &str) -> String {
    let inner = serde_json::to_string(data_json).unwrap();
    format!(r#"{{"status":"success","data":{}}}"#, inner)
}

fn session_expired_body() -> String {
    fixture("session-expired.json")
}

async fn mount(server: &MockServer, method_str: &str, path_expr: &str, body: String) -> MockGuard {
    Mock::given(method(method_str))
        .and(path(path_expr))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount_as_scoped(server)
        .await
}

async fn mount_login_prerequisites(server: &MockServer, probe_body: String) {
    // 用非 scoped mount：函数返回后 mock 需存活到测试结束。
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
    Mock::given(method("GET"))
        .and(path("/my-task.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(probe_body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn login_success_saves_cookie_session() {
    let server = MockServer::start().await;
    mount_login_prerequisites(&server, fixture("task-list.json")).await;
    let _login = Mock::given(method("POST"))
        .and(path("/user-login.html"))
        .and(body_string_contains("account=example-user"))
        .and(body_string_contains("verifyRand=596986874"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("set-cookie", "zentaosid=abc123; path=/")
                .set_body_string("<html><body>ok</body></html>"),
        )
        .mount_as_scoped(&server)
        .await;
    let _probe = mount(&server, "GET", "/my-task.json", fixture("task-list.json")).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9AuthGateway::new(client);
    let session = gateway
        .login(&Credentials {
            account: "example-user".into(),
            password: "secret".into(),
        })
        .await
        .expect("login should succeed");

    assert_eq!(session.server, server.uri());
    assert_eq!(session.cookie, "abc123");
}

#[tokio::test]
async fn login_with_wrong_password_reports_invalid_credentials() {
    let server = MockServer::start().await;
    mount_login_prerequisites(&server, session_expired_body()).await;
    let _login = mount(&server, "POST", "/user-login.html", String::new()).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9AuthGateway::new(client);
    let err = gateway
        .login(&Credentials {
            account: "example-user".into(),
            password: "wrong".into(),
        })
        .await
        .expect_err("login should fail");

    assert!(matches!(err, AuthError::InvalidCredentials));
}

#[tokio::test]
async fn validate_detects_expired_session() {
    let server = MockServer::start().await;
    let _probe = mount(&server, "GET", "/my-task.json", session_expired_body()).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9AuthGateway::new(client);
    let session = Session {
        server: server.uri(),
        cookie: "stale-session-id".into(),
    };
    let err = gateway.validate(&session).await.expect_err("should expire");
    assert!(matches!(err, AuthError::SessionExpired));
}

#[tokio::test]
async fn project_list_parses_contract() {
    let server = MockServer::start().await;
    let _mock = mount(
        &server,
        "GET",
        "/project-index.json",
        fixture("project-list.json"),
    )
    .await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9ProjectGateway::new(client);
    let page = gateway
        .list_projects(ProjectQuery::default())
        .await
        .expect("project list");

    assert_eq!(page.total, 9);
    let first = &page.items[0];
    assert_eq!(first.id.to_string(), "43");
    assert_eq!(first.name, "示例项目-教育");
}

#[tokio::test]
async fn project_list_uses_status_route() {
    let server = MockServer::start().await;
    let _mock = mount(
        &server,
        "GET",
        "/project-all-wait.json",
        fixture("project-list.json"),
    )
    .await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9ProjectGateway::new(client);
    let query = ProjectQuery {
        status: Some("wait".into()),
        ..Default::default()
    };
    let page = gateway.list_projects(query).await.expect("project list");
    assert!(!page.items.is_empty());
}

#[tokio::test]
async fn project_get_returns_detail() {
    let server = MockServer::start().await;
    let _mock = mount(
        &server,
        "GET",
        "/project-view-43.json",
        fixture("project-detail.json"),
    )
    .await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9ProjectGateway::new(client);
    let detail = gateway
        .get_project(EntityId::from("43"))
        .await
        .expect("project detail");

    assert_eq!(detail.name, "示例项目-教育");
    assert_eq!(detail.code, "YDJY");
    assert_eq!(detail.begin.as_deref(), Some("2021-09-28"));
}

#[tokio::test]
async fn project_get_not_found_and_forbidden() {
    let server = MockServer::start().await;
    let _mock = mount(
        &server,
        "GET",
        "/project-view-999.json",
        fixture("not-found.json"),
    )
    .await;
    let _mock2 = mount(
        &server,
        "GET",
        "/project-view-888.json",
        fixture("unauthorized.json"),
    )
    .await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9ProjectGateway::new(client);
    assert!(matches!(
        gateway.get_project(EntityId::from("999")).await,
        Err(QueryError::NotFound)
    ));
    assert!(matches!(
        gateway.get_project(EntityId::from("888")).await,
        Err(QueryError::Forbidden)
    ));
}

#[tokio::test]
async fn task_list_parses_contract() {
    let server = MockServer::start().await;
    let _mock = mount(&server, "GET", "/my-task.json", fixture("task-list.json")).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9TaskGateway::new(client);
    let page = gateway
        .list_tasks(TaskQuery::default())
        .await
        .expect("task list");

    assert_eq!(page.total, 3);
    assert_eq!(page.items.len(), 1);
    let task = &page.items[0];
    assert_eq!(task.id.to_string(), "947");
    assert_eq!(task.project_name, "示例项目-教育");
    assert_eq!(task.status.to_string(), "wait");
}

#[tokio::test]
async fn task_list_filters_status_locally() {
    let server = MockServer::start().await;
    let _mock = mount(&server, "GET", "/my-task.json", fixture("task-list.json")).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9TaskGateway::new(client);
    let query = TaskQuery {
        status: Some("doing".into()),
        ..Default::default()
    };
    let page = gateway.list_tasks(query).await.expect("task list");
    assert!(page.items.is_empty());
}

#[tokio::test]
async fn task_list_rejects_unsupported_assignee() {
    let server = MockServer::start().await;
    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9TaskGateway::new(client);
    let query = TaskQuery {
        assigned_to: Some("someone-else".into()),
        ..Default::default()
    };
    assert!(matches!(
        gateway.list_tasks(query).await,
        Err(QueryError::InvalidParameter(_))
    ));
}

#[tokio::test]
async fn task_get_returns_detail() {
    let server = MockServer::start().await;
    let _mock = mount(
        &server,
        "GET",
        "/task-view-947.json",
        fixture("task-detail.json"),
    )
    .await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9TaskGateway::new(client);
    let detail = gateway
        .get_task(EntityId::from("947"))
        .await
        .expect("task detail");

    assert_eq!(detail.name, "示例任务-退款处理");
    assert_eq!(detail.project_name, "示例项目-教育");
    assert_eq!(detail.desc, "示例描述");
    assert_eq!(detail.deadline.as_deref(), Some("2026-06-12"));
}

#[tokio::test]
async fn bug_list_parses_contract() {
    let server = MockServer::start().await;
    let _mock = mount(
        &server,
        "GET",
        "/my-bug-assignedTo.json",
        fixture("bug-list.json"),
    )
    .await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9BugGateway::new(client);
    let page = gateway
        .list_bugs(BugQuery::default())
        .await
        .expect("bug list");

    assert_eq!(page.total, 5);
    assert_eq!(page.items.len(), 1);
    let bug = &page.items[0];
    assert_eq!(bug.id.to_string(), "41292");
    assert_eq!(bug.title, "示例Bug-界面调整");
    assert_eq!(bug.severity.to_string(), "3");
}

#[tokio::test]
async fn bug_get_returns_detail() {
    let server = MockServer::start().await;
    let _mock = mount(
        &server,
        "GET",
        "/bug-view-41292.json",
        fixture("bug-detail.json"),
    )
    .await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9BugGateway::new(client);
    let detail = gateway
        .get_bug(EntityId::from("41292"))
        .await
        .expect("bug detail");

    assert_eq!(detail.title, "示例Bug-界面调整");
    assert_eq!(detail.product_name, "示例产品");
    assert_eq!(detail.steps, "示例重现步骤");
}

#[tokio::test]
async fn bug_get_extracts_steps_images() {
    let server = MockServer::start().await;
    let data = wrap(
        r#"{"title":"t","productName":"示例产品","bug":{"id":"41824","title":"带图Bug","status":"active","severity":"3","pri":"3","steps":"<p>步骤1</p><p><img src=\"data/upload/2026/08/a.png\" alt=\"\" /></p><p><img src='https://cdn.x.com/b.png'></p>"}}"#,
    );
    let _mock = mount(&server, "GET", "/bug-view-41824.json", data).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9BugGateway::new(client);
    let detail = gateway
        .get_bug(EntityId::from("41824"))
        .await
        .expect("bug detail with images");

    assert_eq!(detail.steps, "步骤1");
    assert_eq!(
        detail.steps_images,
        vec![
            format!("{}/data/upload/2026/08/a.png", server.uri()),
            "https://cdn.x.com/b.png".to_string()
        ]
    );
}

#[tokio::test]
async fn task_get_extracts_desc_images() {
    let server = MockServer::start().await;
    let data = wrap(
        r#"{"task":{"id":"1001","project":"7","name":"带图任务","status":"doing","pri":"2","desc":"<p>说明</p><img src=\"/data/upload/x.jpg\"/>","openedBy":"u","estimate":"1","consumed":"0","left":"1"}}"#,
    );
    let _mock = mount(&server, "GET", "/task-view-1001.json", data).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9TaskGateway::new(client);
    let detail = gateway
        .get_task(EntityId::from("1001"))
        .await
        .expect("task detail with images");

    assert_eq!(detail.desc, "说明");
    assert_eq!(
        detail.desc_images,
        vec![format!("{}/data/upload/x.jpg", server.uri())]
    );
}

#[tokio::test]
async fn project_get_extracts_desc_images() {
    let server = MockServer::start().await;
    let data = wrap(
        r#"{"project":{"id":"101","code":"P","name":"带图项目","status":"doing","desc":"<p>项目说明</p><p><img src=\"data/upload/p.png\"/></p>","PM":"pm1","begin":"2026-01-01","end":"0000-00-00"}}"#,
    );
    let _mock = mount(&server, "GET", "/project-view-101.json", data).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9ProjectGateway::new(client);
    let detail = gateway
        .get_project(EntityId::from("101"))
        .await
        .expect("project detail with images");

    assert_eq!(detail.desc, "项目说明");
    assert_eq!(
        detail.desc_images,
        vec![format!("{}/data/upload/p.png", server.uri())]
    );
}

#[tokio::test]
async fn empty_lists_succeed_with_empty_page() {
    let server = MockServer::start().await;
    let empty_tasks = wrap(
        r#"{"title":"我的地盘-我的任务","tasks":[],"pager":{"recTotal":0,"recPerPage":20,"pageTotal":0,"pageID":1}}"#,
    );
    let _mock = mount(&server, "GET", "/my-task.json", empty_tasks).await;

    let client = ZentaoV9Client::new(server.uri()).unwrap();
    let gateway = ZentaoV9TaskGateway::new(client);
    let page = gateway
        .list_tasks(TaskQuery::default())
        .await
        .expect("empty list should succeed");
    assert!(page.items.is_empty());
    assert_eq!(page.total, 0);
}

#[tokio::test]
async fn network_timeout_maps_to_remote_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/my-task.json"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(500)))
        .mount(&server)
        .await;

    let client = ZentaoV9Client::with_timeout(server.uri(), Duration::from_millis(50)).unwrap();
    let gateway = ZentaoV9TaskGateway::new(client);
    assert!(matches!(
        gateway.list_tasks(TaskQuery::default()).await,
        Err(QueryError::Remote(_))
    ));
}
