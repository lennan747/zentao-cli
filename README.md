# zentao-cli

禅道v9.0.3 命令行客户端。查询当前账号可见的项目、任务和 Bug，并支持对任务与 Bug 的写操作（创建、编辑、指派、状态流转、评论，默认交互确认）。

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

# 写操作（任务）
zentao-cli task create <project-id> --name <名称> [--pri 1-4] [--assigned-to <账号>] [--deadline YYYY-MM-DD] ...
zentao-cli task edit <id> [--name ...] [--desc ...] [--assigned-to ...] [--status wait|doing|done|pause|cancel|closed] [--deadline ...] [--comment ...]
zentao-cli task assign <id> <账号> [--comment ...]
zentao-cli task start <id> [--left <剩余工时>] [--consumed <小时>] [--real-started YYYY-MM-DD HH:MM:SS] [--comment ...]
zentao-cli task finish <id> --consumed <本次耗时> [--finished-date YYYY-MM-DD] [--assigned-to ...] [--comment ...]
zentao-cli task cancel <id> [--comment ...]
zentao-cli task close <id> [--comment ...]
zentao-cli task activate <id> [--comment ...]
zentao-cli task comment <id> --comment <内容>

# 写操作（Bug）
zentao-cli bug create <product-id> --title <标题> [--severity 1-4] [--pri 0-4] [--assigned-to ...] ...
zentao-cli bug edit <id> [--title ...] [--severity ...] [--assigned-to ...] [--status active|resolved|closed] ...
zentao-cli bug assign <id> <账号> [--comment ...]
zentao-cli bug resolve <id> [--resolution fixed|bydesign|duplicate|postponed|willnotfix|notrepro|...] [--resolved-build <build>] [--comment ...]
zentao-cli bug activate <id> [--assigned-to ...] [--comment ...]
zentao-cli bug close <id> [--comment ...]
zentao-cli bug confirm <id> [--comment ...]
zentao-cli bug comment <id> --comment <内容>
```

全局选项：

- `--profile <name>`：多环境配置（默认 `default`）。
- `--format table|json`：输出格式（默认 table）。`json` 输出合法 JSON，无表格装饰。
- `-v, --verbose`：输出诊断日志到 stderr。
- `ZENTAO_CLI_HOME`：覆盖配置目录（测试/脚本用）。

写操作安全护栏：

- 所有写命令默认打印操作摘要并要求交互确认（`[y/N]`）。
- `--yes`：跳过确认直接执行（脚本场景慎用）。
- `--dry-run`：只显示将提交的字段，不发出请求。
- 无终端且未给 `--yes`/`--dry-run`：拒绝执行。
- `--dry-run`/摘要里显示的字段是**将提交的表单内容**；编辑类命令会连同当前字段基线一并提交（旧版接口未提交的字段会被服务端清空，这是实例行为，非 CLI 缺陷）。
- 状态流转命令（start/finish/cancel/close/activate）会先回读任务当前状态做前置校验，状态不符直接拒绝、不发出写请求。
- **`task start` 特别防护**：禅道把「开始且剩余工时为 0」当作「完成」并指派回创建人。CLI 强制 `left > 0`（`--left` 给值或沿用任务当前剩余），否则拒绝执行；任务已无剩余请直接用 `task finish`。
- `task finish` 的 `--consumed` 必填且必须大于 0；CLI 自动带上任务之前的总计消耗作基线，避免服务端误报“总计消耗必须大于之前消耗”。

过滤能力说明（以旧版接口实际能力为准）：

- `project list --status`：wait/doing/done/suspended/closed，走服务端 `/project-all-{status}.json`。
- `task list`：数据来自「我的任务」；`--status` 为本地过滤；`--assigned-to` 仅支持 `me`。
- `bug list`：数据来自「指派给我」；`--status` 为本地过滤；`--assigned-to` 仅支持 `me`。
- `project list` 只返回 ID 和名称（旧版接口限制），完整字段用 `project get`。
- 部分写操作受实例权限控制（如任务 close/activate/assignTo、Bug close/confirm/assignTo 可能被拒绝），CLI 会把服务端拒绝原因原样透出（`user-deny-*`）。

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
