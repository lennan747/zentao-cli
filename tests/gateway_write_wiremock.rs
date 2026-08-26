//! 写操作网关 wiremock 契约测试：覆盖提交、校验失败、无权限与评论响应形态。

use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// 断言请求体不包含某子串。
struct NotContains(String);

impl Match for NotContains {
    fn matches(&self, request: &Request) -> bool {
        !String::from_utf8_lossy(&request.body).contains(&self.0)
    }
}

use zentao_cli::adapters::zentao_v9::{ZentaoV9BugGateway, ZentaoV9Client, ZentaoV9TaskGateway};
use zentao_cli::application::{BugGateway, TaskGateway};
use zentao_cli::domain::{
    BugDraft, BugEdit, BugResolveParams, EntityId, QueryError, TaskDraft, TaskEdit,
    TaskFinishParams, TaskNoteParams,
};

const RELOAD_BODY: &str = "<html><meta charset='utf-8'/><style>body{background:white}</style><script>if(parent !== window) parent.location.reload(true);\n</script>";

fn wrap(data_json: &str) -> String {
    let inner = serde_json::to_string(data_json).unwrap();
    format!(r#"{{"status":"success","data":{}}}"#, inner)
}

fn locate(url: &str) -> String {
    wrap(&format!(r#"{{"locate":"https://x{}"}}"#, url))
}

async fn client(server: &MockServer) -> ZentaoV9Client {
    let client = ZentaoV9Client::new(server.uri()).unwrap();
    client.import_cookies("abc123").unwrap();
    client
}

#[tokio::test]
async fn task_edit_posts_full_baseline_with_overrides() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/task-view-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(task_detail_fixture()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/task-edit-946.json"))
        .and(body_string_contains("deadline=2026-09-01"))
        .and(body_string_contains("comment=%E8%B0%83%E6%95%B4%E6%8E%92%E6%9C%9F"))
        // 基线字段必须一并提交，否则旧版服务端会清空未提交字段
        .and(body_string_contains("consumed=0"))
        .and(body_string_contains("mailto%5B%5D=user1"))
        // status 不随基线提交（工作流规则：doing+剩余=0 时要求 done）
        .and(NotContains("status=".into()))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/task-view-946.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    gateway
        .edit_task(
            EntityId::from("946"),
            TaskEdit {
                deadline: Some("2026-09-01".into()),
                comment: Some("调整排期".into()),
                ..Default::default()
            },
        )
        .await
        .expect("edit should succeed");
}

#[tokio::test]
async fn task_edit_with_team_posts_team_baseline() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/task-view-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            fixture_content("task-detail-with-team.json"),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/task-edit-946.json"))
        .and(body_string_contains("multiple=1"))
        .and(body_string_contains("team%5B%5D=user1"))
        .and(body_string_contains("teamConsumed%5B%5D=1.00"))
        .and(body_string_contains("teamLeft%5B%5D=0.00"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/task-view-946.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    gateway
        .edit_task(
            EntityId::from("946"),
            TaskEdit {
                comment: Some("团队成员基线".into()),
                ..Default::default()
            },
        )
        .await
        .expect("edit should succeed");
}

#[tokio::test]
async fn task_edit_with_explicit_status_posts_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/task-view-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(task_detail_fixture()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/task-edit-946.json"))
        .and(body_string_contains("status=done"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/task-view-946.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    gateway
        .edit_task(
            EntityId::from("946"),
            TaskEdit {
                status: Some("done".into()),
                ..Default::default()
            },
        )
        .await
        .expect("edit should succeed");
}

#[tokio::test]
async fn task_finish_validation_failure_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/task-finish-978.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            wrap(r#"{"result":"fail","message":["\"本次消耗\"不能为0"]}"#),
        ))
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    let err = gateway
        .finish_task(
            EntityId::from("978"),
            TaskFinishParams {
                current_consumed: Some("0".into()),
                ..Default::default()
            },
        )
        .await
        .expect_err("finish should be rejected");
    match err {
        QueryError::Rejected(msg) => assert!(msg.contains("本次消耗")),
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[tokio::test]
async fn task_close_without_permission_is_forbidden() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/task-close-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate(
            "/user-deny-task-close.json",
        )))
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    let err = gateway
        .close_task(EntityId::from("946"), TaskNoteParams::default())
        .await
        .expect_err("close should be forbidden");
    assert!(matches!(err, QueryError::Forbidden));
}

#[tokio::test]
async fn comment_success_uses_alert_response_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/action-comment-task-946.html"))
        .and(body_string_contains("comment=%E5%8A%A0%E6%B2%B9"))
        .respond_with(ResponseTemplate::new(200).set_body_string(RELOAD_BODY))
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    gateway
        .comment_task(EntityId::from("946"), "加油")
        .await
        .expect("comment should succeed");
}

#[tokio::test]
async fn comment_failure_alert_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/action-comment-task-946.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<html><script>alert('\"备注\"不能为空\\n')\n</script>"),
        )
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    let err = gateway
        .comment_task(EntityId::from("946"), "x")
        .await
        .expect_err("comment should fail");
    match err {
        QueryError::Rejected(msg) => assert!(msg.contains("备注")),
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[tokio::test]
async fn task_create_posts_form_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/task-create-43.json"))
        .and(body_string_contains("name=test-task"))
        .and(body_string_contains("assignedTo%5B%5D=demo-user"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/my-task.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    gateway
        .create_task(
            EntityId::from("43"),
            TaskDraft {
                name: "test-task".into(),
                assigned_to: Some("demo-user".into()),
                ..Default::default()
            },
        )
        .await
        .expect("create should succeed");
}

#[tokio::test]
async fn bug_resolve_submits_status_and_resolution() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bug-resolve-41292.json"))
        .and(body_string_contains("status=resolved"))
        .and(body_string_contains("resolution=fixed"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/my-bug-assignedTo.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9BugGateway::new(client(&server).await);
    gateway
        .resolve_bug(
            EntityId::from("41292"),
            BugResolveParams {
                resolution: Some("fixed".into()),
                ..Default::default()
            },
        )
        .await
        .expect("resolve should succeed");
}

#[tokio::test]
async fn bug_create_posts_form_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bug-create-10-0-0.json"))
        .and(body_string_contains("title=test-bug"))
        .and(body_string_contains("severity=3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/my-bug-assignedTo.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9BugGateway::new(client(&server).await);
    gateway
        .create_bug(
            EntityId::from("10"),
            BugDraft {
                title: "test-bug".into(),
                severity: Some("3".into()),
                ..Default::default()
            },
        )
        .await
        .expect("create should succeed");
}

#[tokio::test]
async fn bug_edit_fetches_opened_build_baseline() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/bug-view-41292.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(bug_detail_fixture()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/bug-edit-41292.json"))
        .and(body_string_contains("severity=2"))
        .and(body_string_contains("openedBuild%5B%5D=216"))
        // 基线字段必须一并提交（旧版空提交会清场）
        .and(body_string_contains("product=10"))
        // status 与任务同理不随基线提交
        .and(NotContains("status=".into()))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/bug-view-41292.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9BugGateway::new(client(&server).await);
    gateway
        .edit_bug(
            EntityId::from("41292"),
            BugEdit {
                severity: Some("2".into()),
                ..Default::default()
            },
        )
        .await
        .expect("edit should succeed");
}

#[tokio::test]
async fn bug_edit_with_explicit_opened_build_overrides_baseline() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/bug-view-41292.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(bug_detail_fixture()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/bug-edit-41292.json"))
        .and(body_string_contains("openedBuild%5B%5D=229"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/bug-view-41292.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9BugGateway::new(client(&server).await);
    gateway
        .edit_bug(
            EntityId::from("41292"),
            BugEdit {
                severity: Some("2".into()),
                opened_build: Some("229".into()),
                ..Default::default()
            },
        )
        .await
        .expect("edit should succeed");
}

#[tokio::test]
async fn task_edit_fetches_consumed_baseline() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/task-view-946.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(task_detail_fixture()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/task-edit-946.json"))
        .and(body_string_contains("deadline=2026-09-01"))
        .and(body_string_contains("consumed=0"))
        .and(body_string_contains("left=0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(locate("/task-view-946.json")))
        .expect(1)
        .mount(&server)
        .await;

    let gateway = ZentaoV9TaskGateway::new(client(&server).await);
    gateway
        .edit_task(
            EntityId::from("946"),
            TaskEdit {
                deadline: Some("2026-09-01".into()),
                ..Default::default()
            },
        )
        .await
        .expect("edit should succeed");
}

fn bug_detail_fixture() -> String {
    fixture_content("bug-detail.json")
}

fn task_detail_fixture() -> String {
    fixture_content("task-detail.json")
}

fn fixture_content(name: &str) -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    std::fs::read_to_string(path).unwrap()
}