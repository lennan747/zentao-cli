# 贡献指南

感谢你考虑为 zentao-cli 做贡献！本文说明如何本地开发、测试、提交 PR。

## 环境要求

- Rust >= 1.80（见 `Cargo.toml` 的 `rust-version`）
- 建议安装 `rustfmt` 与 `clippy` 组件

## 本地开发

```bash
git clone https://github.com/lennan747/zentao-cli.git
cd zentao-cli

cargo build          # 构建
cargo test           # 单元 + wiremock 契约 + CLI 端到端测试
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
```

## 代码结构

```text
src/cli/            参数解析、输出格式、退出码
src/application/    端口（trait）：认证与各类查询
src/domain/         DTO、枚举、错误模型（不依赖 clap/reqwest）
src/adapters/zentao_v9/  禅道 v9.0.3 旧版 .json 接口唯一适配层
src/infrastructure/ 配置、会话存储、日志
```

分层约定：`domain` 不依赖 `clap`/`reqwest`/文件系统/具体路由；`adapters/zentao_v9` 是唯一知道旧版路由与响应结构的模块。

## 测试与门禁

提交 PR 前请确保全部通过（CI 也会执行同样检查）：

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## 提交信息约定

- 格式：`类型: 简述`（中文），例如 `feat: 支持 task 评论`、`fix: 修复会话过期误判`、`docs: 更新安装说明`
- 类型：`feat` / `fix` / `docs` / `refactor` / `test` / `chore` / `ci`
- 一个提交只做一件事

## 提交流程

1. Fork 本仓库并创建功能分支（`feat/xxx` 或 `fix/xxx`）。
2. 提交前运行本地门禁。
3. 更新 `CHANGELOG.md`（`Unreleased` 部分）。
4. 推送分支并提交 Pull Request，描述改动动机、影响范围与验证方式。
5. 等待 CI 通过与 review。

## 安全相关约定

- **不得提交**密码、Cookie、会话文件、`.env` 或任何真实实例地址/账号（历史中已发生过内网信息泄漏并重写清除，请务必用占位符，如 `https://zentao.example.com`、`demo-user`）。
- 测试 fixture 统一使用脱敏数据（`示例项目`、`示例任务`、`example-user` 等）。
- 真实环境冒烟严格限定只读，写操作仅在你自己的实例账号与对象上、且经你授权时执行。

## 报告问题

- Bug：使用 [Issue 模板](https://github.com/lennan747/zentao-cli/issues/new?template=bug_report.md)，附复现步骤与 `--verbose` 日志（日志不含密码/Cookie）。
- 功能请求：使用 [功能请求模板](https://github.com/lennan747/zentao-cli/issues/new?template=feature_request.md)。
- 安全漏洞：见 [SECURITY.md](SECURITY.md)，请勿公开披露。
