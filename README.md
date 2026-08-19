# zentao-cli

禅道v9.0.3 命令行客户端。只读查询当前账号可见的项目、任务和 Bug。

## 安装

```bash
# 方式一：本地安装
cargo install --path .

# 方式二：直接使用 release 二进制
cargo build --release --locked
./target/release/zentao-cli --version
```

要求 Rust >= 1.80。二进制名为 `zentao-cli`。

## 快速开始

```bash
# 登录（密码无回显输入，不进入命令行参数和日志）
zentao-cli login --server https://zentao.example.com --account <你的账号>

# 查询
zentao-cli project list
zentao-cli project get 43
zentao-cli task list
zentao-cli task get 971
zentao-cli bug list --assigned-to me
zentao-cli bug get 41292

# 退出登录
zentao-cli logout
```

## 命令

```text
zentao-cli login [--server <url>] [--account <account>]
zentao-cli logout
zentao-cli project list [--status <status>]
zentao-cli project get <id>
zentao-cli task list [--assigned-to me] [--status <status>]
zentao-cli task get <id>
zentao-cli bug list [--assigned-to me] [--status <status>]
zentao-cli bug get <id>
```

全局选项：

- `--profile <name>`：多环境配置（默认 `default`）。
- `--format table|json`：输出格式（默认 table）。`json` 输出合法 JSON，无表格装饰。
- `-v, --verbose`：输出诊断日志到 stderr。
- `ZENTAO_CLI_HOME`：覆盖配置目录（测试/脚本用）。

过滤能力说明（以旧版接口实际能力为准）：

- `project list --status`：wait/doing/done/suspended/closed，走服务端 `/project-all-{status}.json`。
- `task list`：数据来自「我的任务」；`--status` 为本地过滤；`--assigned-to` 仅支持 `me`。
- `bug list`：数据来自「指派给我」；`--status` 为本地过滤；`--assigned-to` 仅支持 `me`。
- `project list` 只返回 ID 和名称（旧版接口限制），完整字段用 `project get`。

## 配置与凭据安全

- 配置保存于平台标准用户配置目录（Linux: `~/.config/zentao-cli/config.toml`），只记录服务器地址和账号，不保存密码。
- Session Cookie 单独保存为 `session-<profile>.json`，权限 `0o600`。
- 密码仅在登录时经无回显终端读取；无 TTY（管道/脚本）时退化为 stdin 读取并给出警告。
- 登录采用旧版表单协议：`verifyRand` + `MD5(MD5(密码)+verifyRand)`，会话只依赖 `zentaosid` Cookie。

## 输出字段

JSON 输出字段即内部稳定 DTO：

- `ProjectSummary`：`id`、`name`
- `ProjectDetail`：+ `code`、`status`、`desc`、`pm`、`begin`、`end`
- `TaskSummary`：`id`、`project_id`、`project_name`、`name`、`status`、`priority`、`assigned_to`、`deadline`
- `TaskDetail`：+ `desc`、`opened_by`、`opened_date`、`estimate`、`consumed`、`left`
- `BugSummary`：`id`、`product_id`、`project_id`、`title`、`status`、`severity`、`priority`、`assigned_to`、`opened_by`
- `BugDetail`：+ `product_name`、`steps`

列表输出为 `Page` 结构：`items`、`total`、`page`、`per_page`、`total_pages`。

## 退出码

| 退出码 | 含义 |
|---|---|
| 0 | 成功 |
| 3 | 认证/会话错误（未登录、凭据错误、会话过期） |
| 4 | 资源不存在或无权限 |
| 6 | 网络/远端/解析等其他查询错误 |
| 7 | 内部错误 |

## 故障排查

- `会话已过期，请重新登录`：重新执行 `login`。
- `请求失败/HTTP 错误`：检查网络、服务器地址和 TLS。
- `远端响应无法解析`：目标实例可能已升级或结构变化，需更新适配层。
- `--verbose` 查看详细日志（日志不包含密码和 Cookie）。

## 卸载

```bash
cargo uninstall zentao-cli   # 若用 cargo install 安装
rm -rf ~/.config/zentao-cli  # 配置与会话
```

## 开发

```bash
cargo build
cargo test              # 单元 + wiremock 契约测试 + CLI 端到端测试
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
```

架构与测试边界见 `tasks/zhongqi-zentao-cli/design/rust-cli-framework.md`。

分层结构：

```text
src/cli/            参数解析、输出格式、退出码
src/application/    端口（trait）：认证与三类查询
src/domain/         DTO、枚举、错误模型（不依赖 clap/reqwest）
src/adapters/zentao_v9/  禅道 9.0.3 旧版 .json 接口唯一适配层
src/infrastructure/ 配置、会话存储、日志
```
