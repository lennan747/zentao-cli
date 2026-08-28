# 变更日志

本项目遵循[语义化版本](https://semver.org/lang/zh-CN/)。所有值得注意的变更记录于此。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。

## [Unreleased]

## [0.1.0] - 2026-08-28

### 新增

- 只读查询：`project` / `task` / `bug` 的 `list` 与 `get`，支持状态过滤与 `--format json`。
- 登录会话：旧版表单协议（`verifyRand` + `MD5(MD5(密码)+verifyRand)`），密码不落盘、不进命令行与日志。
- 写操作：任务与 Bug 的创建、编辑、指派、状态流转与评论，默认交互确认（`--yes` / `--dry-run` 可选）。
- 安全护栏：编辑全量基线提交、状态前置校验、`task start` 强制 left>0、`task finish` 强制 consumed>0。
- 多环境配置：`config` 子命令与 `--profile` 多套服务器/账号。
- 输出美化：极简表格、中文列头与字段名、状态中文标签与配色、长文本折行与超长截断、HTML 实体解码。
- CI 与发布：GitHub Actions 门禁 + tag 触发发布多平台二进制与 `SHA256SUMS`。
- 一键安装：`install.sh`（下载 + SHA256 校验 + 安装到 `~/.local/bin`）。

[Unreleased]: https://github.com/lennan747/zentao-cli/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lennan747/zentao-cli/releases/tag/v0.1.0
