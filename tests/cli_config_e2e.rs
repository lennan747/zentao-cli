//! config 子命令端到端测试：path/show/set/init 在隔离配置目录下运行。

use assert_cmd::Command;
use tempfile::TempDir;

fn zentao(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("zentao-cli").unwrap();
    cmd.env("ZENTAO_CLI_HOME", home.path());
    cmd
}

#[test]
fn config_path_prints_config_file_location() {
    let home = TempDir::new().unwrap();
    zentao(&home)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicates::str::contains("config.toml"));
}

#[test]
fn config_init_creates_template_once() {
    let home = TempDir::new().unwrap();
    zentao(&home)
        .args(["config", "init"])
        .assert()
        .success()
        .stdout(predicates::str::contains("已初始化"));
    zentao(&home)
        .args(["config", "init"])
        .assert()
        .success()
        .stdout(predicates::str::contains("已存在"));
    assert!(home.path().join("config.toml").exists());
}

#[test]
fn config_set_and_show_roundtrip() {
    let home = TempDir::new().unwrap();
    zentao(&home)
        .args(["config", "set", "server", "https://x.com/"])
        .assert()
        .success();
    zentao(&home)
        .args(["config", "set", "account", "demo-user"])
        .assert()
        .success();
    zentao(&home)
        .args(["config", "set", "timeout", "60"])
        .assert()
        .success();

    zentao(&home)
        .args(["config", "show", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"server\": \"https://x.com\""))
        .stdout(predicates::str::contains("\"account\": \"demo-user\""))
        .stdout(predicates::str::contains("\"timeout_seconds\": 60"));
}

#[test]
fn config_set_rejects_unknown_key() {
    let home = TempDir::new().unwrap();
    zentao(&home)
        .args(["config", "set", "password", "x"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("不支持的配置键"));
}

#[test]
fn config_set_rejects_invalid_timeout() {
    let home = TempDir::new().unwrap();
    zentao(&home)
        .args(["config", "set", "timeout", "abc"])
        .assert()
        .code(6)
        .stderr(predicates::str::contains("非负整数"));
}
