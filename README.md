# zentao-cli

禅道v9.0.3 命令行客户端。

## 构建要求

- Rust >= 1.80
- 已安装 `cargo`

## 开发命令

```bash
# 构建
cargo build

# 运行测试
cargo test

# 静态检查
cargo clippy -- -D warnings

# 发布构建
cargo build --release
```

## 计划命令

```text
zentao login [--server <url>] [--account <account>]
zentao logout
zentao project list [--status <status>]
zentao project get <id>
zentao task list [--assigned-to <account>] [--status <status>]
zentao task get <id>
zentao bug list [--assigned-to <account>] [--status <status>]
zentao bug get <id>
```

## 配置

配置保存在平台标准用户配置目录（Linux: `~/.config/zentao-cli/config.toml`）。
Session Cookie 单独保存在 `session-<profile>.json`，权限为 `0o600`。

## 架构

见 `tasks/zhongqi-zentao-cli/design/rust-cli-framework.md`。
