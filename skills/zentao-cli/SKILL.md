---
name: zentao-cli
description: 通过 zentao-cli 命令行操作禅道（禅道v9.0.3 旧版 .json 接口）。当用户提到禅道、zentao、任务、task、Bug、缺陷、项目、project、工时、预估、指派、评论、状态流转，或要求查询/创建/编辑/关闭/解决任务与 Bug 时触发。Use when the user asks to query or modify ZenTao tasks, bugs, or projects.
---

# zentao-cli（禅道 CLI）Skill

用 `zentao-cli` 帮用户操作禅道。本工具对接禅道 v9.0.3 的旧版 `.json` 接口，覆盖 `login/logout/config/project/task/bug` 六类命令。

## 一、前置检查（开始前必须做）

1. **确认已安装**：`command -v zentao-cli`；未安装则告知用户用一键安装：
   ```bash
   curl -fsSL https://raw.githubusercontent.com/lennan747/zentao-cli/master/install.sh | sh
   ```
2. **确认已配置**：`zentao-cli config show` 看 `server`/`account` 是否已填。
   - 未配置 → `zentao-cli config set server <服务器地址>`、`zentao-cli config set account <账号>`
   - 多套环境用 `--profile <name>` 区分（默认 `default`）。
3. **确认已登录**：会话文件存在才算登录（会话过期/未登录表现为退出码 3）。
   - 登录命令：`zentao-cli login [-s <地址>] [-a <账号>]`
   - **重要**：`login` 是无回显交互式密码输入（`rpassword`），**AI 无法代输密码**。你必须把命令交给用户，让用户在本终端输入密码；无 TTY 时退化为 stdin 读取。密码不落盘、不进命令行参数与日志。

## 二、全局参数

| 参数 | 说明 |
|---|---|
| `--profile <name>` | 多环境配置，默认 `default` |
| `--format table\|json` | 输出格式，默认 `table`；`json` 输出合法 JSON（仅成功时） |
| `-v, --verbose` | 诊断日志到 stderr（不含密码与 Cookie） |
| `ZENTAO_CLI_HOME` | 环境变量，覆盖配置目录（测试/脚本用） |

## 三、退出码（判断成败的唯一依据）

| 退出码 | 含义 | 处理 |
|---|---|---|
| `0` | 成功 | — |
| `3` | 认证/授权错误：密码错、会话过期、未登录、缺凭据 | 引导用户重新 `login` |
| `4` | 资源不存在 / 无权限 | 确认 ID 是否正确、账号是否有权限 |
| `6` | 参数无效 / 远端校验拒绝 / 响应无法解析 / 网络或远端错误 | 检查参数取值，读错误信息 |
| `7` | 内部错误 | 反馈 bug，附 `-v` 日志（自查不含敏感信息） |

错误输出到 **stderr**（`error: <信息>`，纯文本，即使 `--format json` 也不变），成功输出到 stdout。

## 四、命令速查

### 配置与认证
```bash
zentao-cli config init                     # 初始化配置模板（已存在则不动）
zentao-cli config path                     # 配置文件路径
zentao-cli config show                     # 显示当前配置（无敏感信息）
zentao-cli config set <key> <value>        # key: server / account / timeout(秒)
zentao-cli login [-s 地址] [-a 账号]       # 交互输密码
zentao-cli logout
```

### 项目（只读）
```bash
zentao-cli project list [--status wait|doing|done|suspended|closed|all]
zentao-cli project get <id>
```

### 任务
```bash
zentao-cli task list [-s wait|doing|done|pause|cancel|closed] [-a me]   # 数据来自「我的任务」；-s 本地过滤；-a 仅支持 me
zentao-cli task get <id>
zentao-cli task create <项目ID> --name <名称> \
  [--desc 描述] [--pri 1-4] [--type design|devel|test|study|discuss|ui|affair|misc|production|management] \
  [--estimate 小时] [--est-started YYYY-MM-DD] [--deadline YYYY-MM-DD] \
  [--module 模块ID] [--assigned-to 账号] [--mailto 账号]... 
zentao-cli task edit <id> [--name] [--desc] [--assigned-to] [--pri 0-4] [--type] \
  [--status wait|doing|done|pause|cancel|closed] [--estimate] [--consumed] [--left] \
  [--deadline] [--est-started] [--comment]
zentao-cli task assign <id> <账号> [--comment]
zentao-cli task start <id> --left <小时> [--consumed] [--real-started "YYYY-MM-DD HH:MM:SS"] [--assigned-to] [--comment]
zentao-cli task finish <id> --consumed <小时>   # consumed 必填且 >0
zentao-cli task cancel <id>                     # 取消
zentao-cli task close <id>                      # 关闭
zentao-cli task activate <id>                   # 激活（done/cancel/closed -> wait）
zentao-cli task comment <id> <内容>
```

### Bug
```bash
zentao-cli bug list [-s active|resolved|closed] [-a me]   # 数据来自「指派给我」；-s 本地过滤；-a 仅支持 me
zentao-cli bug get <id>
zentao-cli bug create <产品ID> --title <标题> \
  [--steps 复现步骤] [--module 模块ID] [--project 项目ID] \
  [--severity 1-4] [--pri 0-4] [--assigned-to 账号] \
  [--opened-build 版本] [--deadline YYYY-MM-DD] [--keywords 关键词] \
  [--type codeerror|designchange|newfeature|others|...] [--os all|windows|...]
zentao-cli bug edit <id> ...
zentao-cli bug assign <id> <账号>
zentao-cli bug resolve <id> ...    # -> resolved
zentao-cli bug activate <id>       # resolved/closed -> active
zentao-cli bug close <id>
zentao-cli bug confirm <id>        # 确认 Bug
zentao-cli bug comment <id> <内容>
```

## 五、AI 使用准则（务必遵守）

### 读操作
- 一律加 `--format json`，用退出码判断成败；成功时解析 stdout 的 JSON。
- `list` 的 JSON 结构统一为分页对象：
  ```json
  { "items": [ ... ], "total": 3, "page": 1, "per_page": 20, "total_pages": 1 }
  ```
- `get` 的 JSON 是对象详情本体（字段随实体而异，直接按返回字段读）。
- 需要结构化判断时（如“是否存在某任务”），读 `total` 或遍历 `items`，不要解析 `table` 输出。

### 写操作（铁律，按顺序）
1. **先 `get <id>` 拉全量**，了解当前字段基线。
2. **先 `--dry-run` 预览**：`zentao-cli task edit <id> ... --dry-run`，把将提交的字段展示给用户。
3. **向用户展示摘要并征得明确确认**后，才去掉 `--dry-run` 执行。
4. **不得在未经用户同意时使用 `--yes`**。写命令默认交互确认；无 TTY 时会拒绝执行（除非 `--yes`）。
5. 编辑会连同当前对象全部字段基线一并提交（旧版接口对未提交字段按空值处理），**务必先 get 再 edit**。

### 状态流转约束
- `task start`：`--left` 必须 >0（为 0 时禅道会把“开始”当作“完成”并指派回创建人，CLI 直接拒绝）。
- `task finish`：`--consumed` 必填且 >0。
- `edit --status`：仅在你显式指定时才提交，避免触发服务端工作流校验；能用专用子命令（`start/finish/cancel/close/activate`）就用专用子命令，不要用 `edit --status` 硬改。

### 数据范围
- `task list` 只返回「我的任务」；`bug list` 只返回「指派给我」；`project list` 返回全部可见项目。旧版接口 `-a/--assigned-to` 仅支持 `me`。

## 六、枚举值速查

| 实体 | 字段 | 取值 |
|---|---|---|
| project | status | `wait` 等待 / `doing` 进行中 / `done` 已完成 / `suspended` 已挂起 / `closed` 已关闭 |
| task | status | `wait` 未开始 / `doing` 进行中 / `done` 已完成 / `pause` 已暂停 / `cancel` 已取消 / `closed` 已关闭 |
| task | pri | `1` 最高 / `2` / `3` / `4` 最低（edit 时允许 0） |
| task | type | `design` / `devel` / `test` / `study` / `discuss` / `ui` / `affair` / `misc` / `production` / `management` |
| bug | status | `active` 激活 / `resolved` 已解决 / `closed` 已关闭 |
| bug | severity | `1`-`4` |
| bug | type | `codeerror` 代码错误 / `designchange` 设计变更 / `newfeature` 新功能 / `others` 其他 / … |

## 七、安全与风险声明

- 本工具会被 AI 调用以在禅道上执行写操作，存在误操作风险。**写操作必须经用户明确确认**（见第五部分铁律）。
- **不得**把服务器地址、账号、任务/Bug 内容、会话信息输出到公开渠道或写入仓库提交。
- `-v` 日志不含密码与 Cookie，但粘贴给别人前仍要自查一遍。
- 密码不保存到配置/会话文件；会话文件权限 `0o600`；配置文件位置见 `config path`。

## 八、故障排查

| 现象 | 原因 | 处理 |
|---|---|---|
| 退出码 3 | 未登录 / 会话过期 / 密码错 | `zentao-cli login`（密码需用户输入） |
| 退出码 4 | 找不到资源 / 无权限 | 用 `list` 确认 ID；确认账号权限 |
| 退出码 6 | 参数或远端拒绝 | 核对枚举取值与必填项；重跑并读 stderr |
| `task start` 被拒 | `--left` 为 0 或缺省为 0 | 显式传 `--left >0` |
| `task finish` 被拒 | `--consumed` 缺或 0 | 显式传 `--consumed >0` |
