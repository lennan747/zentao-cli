use std::path::PathBuf;

use zentao_cli::adapters::zentao_v9::response::parse_body;
use zentao_cli::domain::QueryError;

fn fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e))
}

#[test]
fn project_list_envelope_parses() {
    let body = fixture("project-list.json");
    let value = parse_body(&body).expect("should parse project list");
    assert!(value.get("projects").is_some());
}

#[test]
fn task_list_envelope_parses() {
    let body = fixture("task-list.json");
    let value = parse_body(&body).expect("should parse task list");
    assert!(value.get("tasks").is_some());
}

#[test]
fn bug_list_envelope_parses() {
    let body = fixture("bug-list.json");
    let value = parse_body(&body).expect("should parse bug list");
    assert!(value.get("bugs").is_some());
}

#[test]
fn not_found_returns_not_found_error() {
    let body = fixture("not-found.json");
    let err = parse_body(&body).expect_err("should fail for not found");
    assert!(matches!(err, QueryError::NotFound));
}

#[test]
fn unauthorized_returns_forbidden_error() {
    let body = fixture("unauthorized.json");
    let err = parse_body(&body).expect_err("should fail for unauthorized");
    assert!(matches!(err, QueryError::Forbidden));
}

#[test]
fn session_expired_returns_session_expired_error() {
    let body = fixture("session-expired.json");
    let err = parse_body(&body).expect_err("should fail for session expired");
    assert!(matches!(err, QueryError::SessionExpired));
}
