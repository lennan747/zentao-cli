# zentao-cli

[![GitHub Release](https://img.shields.io/github/v/release/lennan747/zentao-cli)](https://github.com/lennan747/zentao-cli/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/lennan747/zentao-cli/ci.yml?branch=master&label=ci)](https://github.com/lennan747/zentao-cli/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

禅道v9.0.3 命令行客户端。查询当前账号可见的项目、任务和 Bug，并支持对任务与 Bug 的写操作（创建、编辑、指派、状态流转、评论，默认交互确认）。

## 特性

- **只读查询**：项目、任务、Bug 的 `list` / `get`，支持状态过滤与 `--format json` 结构化输出。
- **写操作**：任务与 Bug 的创建、编辑、指派、状态流转、评论。
- **安全护栏**：写命令默认打印摘要并交互确认；`--dry-run` 只预览不提交；无 TTY 拒绝执行；密码不进入命令行参数与日志（可选保存到配置，见「配置与凭据安全」）。
- **友好输出**：极简表格、中文列头与字段名、语义配色、长文本折行与超长截断（详见「输出样式」）。
- **多环境配置**：`--profile` 支持多套服务器/账号配置。

## 目录

- [安装](#安装)
- [快速开始](#快速开始)
- [AI Agent Skill](#ai-agent-skill)
- [命令](#命令)
  - [通用](#通用)
  - [配置](#配置)
  - [认证](#认证)
  - [项目](#项目)
  - [任务](#任务)
  - [Bug](#bug)
- [配置与凭据安全](#配置与凭据安全)
- [输出字段](#输出字段)
- [退出码](#退出码)
- [故障排查](#故障排查)
- [卸载](#卸载)
- [开发](#开发)

## 安装

> 要求：Linux / macOS x86_64 或 macOS arm64，支持 SHA256 校验；Windows 请使用 [WSL](https://learn.microsoft.com/windows/wsl/install) 后按 Linux 方式安装，或手动下载对应资产。Rust 源码安装要求 Rust >= 1.80。

### 一键安装（推荐）

从 GitHub Releases 拉取最新二进制，自动校验 SHA256：

```bash
curl -fsSL https://raw.githubusercontent.com/lennan747/zentao-cli/master/install.sh | sh
```

- 安装到 `~/.local/bin/zentao-cli`（可用 `ZENTAO_CLI_INSTALL_DIR` 覆盖）。
- 指定版本：`ZENTAO_CLI_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/lennan747/zentao-cli/master/install.sh | sh`。

### cargo 安装

```bash
cargo install --git https://github.com/lennan747/zentao-cli.git --locked
```

### 手动构建

```bash
git clone https://github.com/lennan747/zentao-cli.git
cd zentao-cli
cargo build --release --locked
./target/release/zentao-cli --version
```

安装后请先 [登录](#login)。

## 快速开始

```bash
# 1. 登录（密码无回显输入，不进入命令行参数和日志；已用 config set password 保存则免输入）
zentao-cli login --server https://zentao.example.com --account <你的账号>

# 2. 查询
zentao-cli project list
zentao-cli project get 101
zentao-cli task list
zentao-cli task get 1001
zentao-cli bug list --assigned-to me
zentao-cli bug get 2001

# 3. 写操作（默认需确认；可用 --dry-run 先预览）
zentao-cli task start 1001 --left 5 --dry-run
zentao-cli task comment 1001 --comment "已确认排期"

# 4. 退出登录
zentao-cli logout
```

## AI Agent Skill

本仓库附带一份面向 AI Agent 的 [Skill](skills/zentao-cli/SKILL.md)，供 Claude Code / opencode 等 AI 编码工具调用 `zentao-cli` 时加载：包含命令速查、JSON 输出契约、退出码表、写操作安全准则（先 `--dry-run`、经用户确认、状态流转约束）与枚举值速查。

### 一句话安装（通过 AI Agent）

复制下面这句给任意 AI Agent（Claude Code、opencode、Cursor、Trae 等），它会自动完成 CLI 与 Skill 的安装：

```
帮我安装 zentao-cli：https://raw.githubusercontent.com/lennan747/zentao-cli/master/docs/install-guide.md
```

```
Install zentao-cli: https://raw.githubusercontent.com/lennan747/zentao-cli/master/docs/install-guide.md
```

完整分步说明见 [docs/install-guide.md](docs/install-guide.md)。

### 手动安装

安装到 opencode（全局技能，改完需**重启 opencode** 生效）：

```bash
mkdir -p ~/.config/opencode/skills
cp -r skills/zentao-cli ~/.config/opencode/skills/
```

安装到 Claude Code：

```bash
cp -r skills/zentao-cli ~/.claude/skills/
```

## 命令

### 通用

**全局选项**（所有子命令均可用）：

| 选项 | 说明 |
|---|---|
| `--profile <name>` | 多环境配置，默认 `default` |
| `--format table\|json` | 输出格式，默认 `table`；`json` 输出合法 JSON |
| `-v, --verbose` | 诊断日志输出到 stderr（不包含密码和 Cookie） |
| `ZENTAO_CLI_HOME` | 环境变量，覆盖配置目录（测试/脚本用） |

**输出样式**：

- 表格使用极简 Unicode 边框（无竖线，仅水平分隔线）；表头深灰底白字加粗；TTY 下按终端宽度自适应列宽。
- 列表列头与 `get` 详情字段名均为中文（如 `ID / 项目 / 名称 / 状态 / 优先级 / 指派给`、`字段 / 值`）；写操作确认摘要的字段名同步中文化。
- 状态列显示中文标签 + 符号并按语义着色：`○ 等待中` 黄、`● 进行中` 青、`✔ 已完成` 绿、`⏸ 已暂停` 黄、`✘ 已取消` 红、`已关闭` 暗（Bug 对应 `● 处理中`/`✔ 已解决`/`已关闭`）；`get` 详情中的 status 取值同样显示中文标签。
- 优先级列：1/2 红色加粗、3/4 绿色（`get` 详情中优先级/严重程度保留数字）。
- 服务端文本中的 HTML 实体（如 `&amp;`）会自动解码；Name/Title 列超长按显示宽度截断为 `...`。
- 成功/警告/错误消息分别以绿/黄/红显示；错误仅在 `-v` 时附带诊断日志。
- 输出被重定向或管道（非 TTY）时自动去掉所有颜色；设置 `NO_COLOR` 环境变量可强制关闭颜色。

**写操作安全护栏**（`create/edit/assign/start/finish/cancel/close/activate/comment/resolve/confirm`）：

| 机制 | 说明 |
|---|---|
| 默认确认 | 打印操作摘要，等待 `[y/N]` 输入 |
| `--yes` | 跳过确认（脚本场景慎用） |
| `--dry-run` | 只显示将提交的字段，不发出请求 |
| 无 TTY 拒绝 | 未连接终端且未给 `--yes`/`--dry-run` 时拒绝执行 |
| 全量基线 | 编辑类命令（task/bug edit）回读当前对象全部字段再覆盖变更，防止旧版接口清空未提交字段 |
| 状态前置校验 | start/finish/cancel/close/activate 先回读当前状态，不符直接拒绝（不发出写请求） |
| `task start` 防护 | left 必须 >0（为 0 时禅道会直接标记完成并指派回创建人），否则拒绝执行 |
| `task finish` 基线 | `--consumed` 必填且 >0，CLI 自动带之前总计消耗作基线，防止误报"总计消耗必须大于之前消耗" |

**过滤能力说明**（以旧版接口实际能力为准）：

| 命令 | 说明 |
|---|---|
| `project list --status` | wait/doing/done/suspended/closed，服务端 `/project-all-{status}.json` 过滤 |
| `task list` | 数据来自「我的任务」；`-s/--status` 本地过滤；`-a/--assigned-to` 仅支持 `me` |
| `bug list` | 数据来自「指派给我」；`-s/--status` 本地过滤；`-a/--assigned-to` 仅支持 `me` |
| 权限 | 部分写操作受实例权限控制（如任务 close/activate/assignTo、Bug close/confirm/assignTo），CLI 以退出码 4 透出 `user-deny-*` |

---

### 配置

登录相关参数（服务器、账号、请求超时）以配置文件形式管理。

#### config path

打印配置文件路径。

```
zentao-cli config path
```

#### config show

显示当前 profile 的配置（密码掩码显示为 `****`）。

```
zentao-cli config show [--profile <name>]
```

```bash
zentao-cli config show
zentao-cli config show --format json
```

#### config set

设置登录参数，作用于当前 `--profile`。

```
zentao-cli config set <key> <value>
```

| key | 说明 |
|---|---|
| `server` | 禅道服务器地址（末尾 `/` 会被去掉） |
| `account` | 登录账号 |
| `timeout` | 请求超时秒数（非负整数，0 表示用默认 30 秒） |
| `password` | 登录密码；明文存于 config.toml（权限 0o600），传空串清除 |

```bash
zentao-cli config set server https://zentao.example.com
zentao-cli config set account <你的账号>
zentao-cli config set timeout 60
zentao-cli config set password <你的密码>   # 保存后 login 免输入、会话过期自动重登
zentao-cli config set password ""          # 清除已保存的密码
```

#### config init

配置文件不存在时生成模板（已存在则不改动）。

```
zentao-cli config init
```

---

### 认证

#### login

登录禅道，获取 `zentaosid` 会话 Cookie。

```
zentao-cli login [--server <url>] [--account <账号>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `-s, --server <url>` | 否 | 禅道地址，缺省用配置文件中已保存的 |
| `-a, --account <账号>` | 否 | 登录账号，缺省用配置文件中已保存的 |

密码取值顺序：① 配置中已保存的密码（免输入）② 无回显终端读取。密码不进入命令行参数与日志；无 TTY 且未保存密码时退化为 stdin 读取并给出警告。

```bash
# 已用 config set password 保存密码 → 免交互
zentao-cli login

# 交互式（密码无回显）
zentao-cli login --server https://zentao.example.com --account <你的账号>

# 脚本（密码走 stdin，会打印警告；未保存密码时）
echo "$PASSWORD" | zentao-cli login --server https://zentao.example.com --account <你的账号>
```

> 查询/写命令遇会话过期（退出码 3）且已保存密码时，会自动重登并重试一次，无需手动干预。

#### logout

删除本地会话文件，不影响远端。

```
zentao-cli logout
```

---

### 项目

#### project list

列出当前账号可见的项目。

```
zentao-cli project list [-s <status>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `-s, --status <status>` | 否 | 状态过滤：wait/doing/done/suspended/closed，不传则返回全部 |

```bash
zentao-cli project list
zentao-cli project list --status doing
```

#### project get

查看项目详情。

```
zentao-cli project get <id>
```

| 参数 | 必填 | 说明 |
|---|---|---|
| `id` | 是 | 项目 ID |

返回字段：id、code、name、status、desc、pm、begin、end。

```bash
zentao-cli project get 101
```

---

### 任务

#### task list

列出「我的任务」。

```
zentao-cli task list [-a me] [-s <status>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `-a, --assigned-to me` | 否 | 指派对象，旧版接口仅支持 `me` |
| `-s, --status <status>` | 否 | 状态过滤：wait/doing/done/pause/cancel/closed，本地过滤 |

```bash
zentao-cli task list
zentao-cli task list --status doing
```

#### task get

查看任务详情。

```
zentao-cli task get <id>
```

返回字段：id、project_id、project_name、name、status、priority、assigned_to、desc、opened_by、opened_date、deadline、estimate、consumed、left。

```bash
zentao-cli task get 1001
```

#### task create

在指定项目下创建任务。

```
zentao-cli task create <project> --name <名称> [选项...]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<project>` | 是 | 项目 ID（位置参数） |
| `--name <名称>` | 是 | 任务名称 |
| `--desc <描述>` | 否 | 任务描述 |
| `--pri <1-4>` | 否 | 优先级：1 最高，4 最低 |
| `--type <TYPE>` | 否 | 类型：design/devel/test/study/discuss/ui/affair/misc/production/management |
| `--estimate <小时>` | 否 | 预计工时 |
| `--est-started <YYYY-MM-DD>` | 否 | 预计开始日期 |
| `--deadline <YYYY-MM-DD>` | 否 | 截止日期 |
| `--module <id>` | 否 | 所属模块 ID，0=根 |
| `--assigned-to <账号>` | 否 | 指派给 |
| `--mailto <账号>` | 否 | 抄送，可多次指定 |

```bash
zentao-cli task create 101 --name "修复登录页样式" --pri 2 --assigned-to <你的账号>
```

#### task edit

编辑任务（提交当前字段基线 + 覆盖变更，仅提交用户显式指定的字段）。

```
zentao-cli task edit <id> [选项...]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | 任务 ID |
| `--name <名称>` | 否 | 任务名称 |
| `--desc <描述>` | 否 | 任务描述 |
| `--assigned-to <账号>` | 否 | 指派人 |
| `--pri <0-4>` | 否 | 优先级 |
| `--type <TYPE>` | 否 | 类型（同 create） |
| `--status <status>` | 否 | 状态：wait/doing/done/pause/cancel/closed。仅当你显式指定时才提交（避免触发工作流校验） |
| `--estimate <小时>` | 否 | 预计工时 |
| `--consumed <小时>` | 否 | 总计消耗 |
| `--left <小时>` | 否 | 剩余工时 |
| `--deadline <YYYY-MM-DD>` | 否 | 截止日期 |
| `--est-started <YYYY-MM-DD>` | 否 | 预计开始日期 |
| `--comment <备注>` | 否 | 附带评论 |

> 编辑会连同当前对象全部字段基线一并提交。禅道旧版接口对未提交的字段会按空值处理（清空），详细说明见「通用」安全护栏。

```bash
zentao-cli task edit 1001 --deadline 2026-06-16 --comment "调整排期"
```

#### task assign

指派任务（通过编辑接口提交 `assignedTo`）。

```
zentao-cli task assign <id> <账号> [--comment <备注>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | 任务 ID |
| `<账号>` | 是 | 指派给（位置参数） |
| `--comment <备注>` | 否 | 附带评论 |

```bash
zentao-cli task assign 1001 <你的账号> --comment "你来跟进"
```

#### task start

开始任务（wait/pause → doing）。

```
zentao-cli task start <id> [选项...]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | 任务 ID |
| `--left <小时>` | **强烈建议** | 剩余工时，**必须 >0**。为 0 时禅道会把"开始"当作"完成"并指派回创建人，CLI 会拒绝执行。缺省沿用任务当前剩余 |
| `--consumed <小时>` | 否 | 本次消耗，缺省 0 |
| `--real-started <YYYY-MM-DD HH:MM:SS>` | 否 | 实际开始时间，缺省用服务端当前时间 |
| `--assigned-to <账号>` | 否 | 开始后指派给，缺省不变 |
| `--comment <备注>` | 否 | 附带评论 |

> 前置条件：仅 wait/pause 状态可开始，否则 CLI 拒绝执行。

```bash
zentao-cli task start 1001 --left 5 --consumed 0.5
```

#### task finish

完成任务（wait/doing → done）。

```
zentao-cli task finish <id> --consumed <本次耗时> [选项...]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | 任务 ID |
| `--consumed <小时>` | 是 | 本次消耗工时，必须 >0 |
| `--finished-date <YYYY-MM-DD>` | 否 | 完成日期，缺省用服务端当前时间 |
| `--assigned-to <账号>` | 否 | 完成后指派给，缺省不变 |
| `--comment <备注>` | 否 | 附带评论 |

> 前置条件：仅 wait/doing 状态可完成。CLI 会自动提交之前的总计消耗作为基线，避免服务端"总计消耗必须大于之前消耗"误报。

```bash
zentao-cli task finish 1001 --consumed 2 --comment "已验收通过"
```

#### task cancel

取消任务（wait/doing/pause → cancel）。

```
zentao-cli task cancel <id> [--comment <备注>]
```

> 前置条件：仅 wait/doing/pause 状态可取消。

```bash
zentao-cli task cancel 1001 --comment "需求变更，不再需要"
```

#### task close

关闭任务（done → closed）。

```
zentao-cli task close <id> [--comment <备注>]
```

> 前置条件：仅 done 状态可关闭。部分账号可能无此权限（透传 `user-deny`）。

```bash
zentao-cli task close 1001 --comment "客户确认完成"
```

#### task activate

激活任务（done/cancel/closed → wait）。

```
zentao-cli task activate <id> [--comment <备注>]
```

> 前置条件：仅 done/cancel/closed 状态可激活。部分账号可能无此权限（透传 `user-deny`）。

```bash
zentao-cli task activate 1001 --comment "需要重新打开"
```

#### task comment

给任务添加评论。

```
zentao-cli task comment <id> --comment <内容>
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | 任务 ID |
| `--comment <内容>` | 是 | 评论内容，不能为空 |

```bash
zentao-cli task comment 1001 --comment "已和客户确认排期"
```

---

### Bug

#### bug list

列出「指派给我」的 Bug。

```
zentao-cli bug list [-a me] [-s <status>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `-a, --assigned-to me` | 否 | 指派对象，旧版接口仅支持 `me` |
| `-s, --status <status>` | 否 | 状态过滤：active/resolved/closed，本地过滤 |

```bash
zentao-cli bug list
zentao-cli bug list --status active
```

#### bug get

查看 Bug 详情。

```
zentao-cli bug get <id>
```

返回字段：id、product_id、product_name、project_id、title、status、severity、priority、assigned_to、opened_by、opened_date、steps。

```bash
zentao-cli bug get 2001
```

#### bug create

在指定产品下创建 Bug。

```
zentao-cli bug create <product> --title <标题> [选项...]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<product>` | 是 | 产品 ID（位置参数） |
| `--title <标题>` | 是 | Bug 标题 |
| `--steps <步骤>` | 否 | 复现步骤 |
| `--module <id>` | 否 | 所属模块 ID，0=根 |
| `--project <id>` | 否 | 所属项目 ID |
| `--severity <1-4>` | 否 | 严重程度：1 最高，4 最低 |
| `--pri <0-4>` | 否 | 优先级 |
| `--assigned-to <账号>` | 否 | 指派给 |
| `--opened-build <build>` | 否 | 影响版本 |
| `--deadline <YYYY-MM-DD>` | 否 | 截止日期 |
| `--keywords <关键词>` | 否 | 关键词 |
| `--type <TYPE>` | 否 | 类型：codeerror/designchange/newfeature/others/optimize/... |
| `--os <OS>` | 否 | 操作系统：all/windows/win10/android/ios/... |
| `--browser <browser>` | 否 | 浏览器：all/ie/chrome/firefox/... |
| `--mailto <账号>` | 否 | 抄送，可多次指定 |

```bash
zentao-cli bug create 201 --title "登录页报 500" --severity 2 --assigned-to <你的账号>
```

#### bug edit

编辑 Bug（提交当前字段基线 + 覆盖变更）。

```
zentao-cli bug edit <id> [选项...]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | Bug ID |
| `--title <标题>` | 否 | 标题 |
| `--steps <步骤>` | 否 | 复现步骤 |
| `--severity <1-4>` | 否 | 严重程度 |
| `--pri <0-4>` | 否 | 优先级 |
| `--assigned-to <账号>` | 否 | 指派人 |
| `--status <status>` | 否 | 状态：active/resolved/closed。仅当显式指定时提交 |
| `--resolution <方案>` | 否 | 解决方案：fixed/bydesign/duplicate/postponed/willnotfix/notrepro/... |
| `--resolved-build <build>` | 否 | 解决版本 |
| `--opened-build <build>` | 否 | 影响版本，缺省沿用当前值 |
| `--deadline <YYYY-MM-DD>` | 否 | 截止日期 |
| `--keywords <关键词>` | 否 | 关键词 |
| `--type <TYPE>` | 否 | 类型（同 create） |
| `--os <OS>` | 否 | 操作系统 |
| `--browser <browser>` | 否 | 浏览器 |
| `--comment <备注>` | 否 | 附带评论 |

> 同 task edit，编辑会连同当前字段基线一并提交。

```bash
zentao-cli bug edit 2001 --severity 1 --comment "升级严重程度"
```

#### bug assign

指派 Bug（通过编辑接口提交 `assignedTo`）。

```
zentao-cli bug assign <id> <账号> [--comment <备注>]
```

```bash
zentao-cli bug assign 2001 <你的账号>
```

#### bug resolve

解决 Bug（→ resolved）。

```
zentao-cli bug resolve <id> [选项...]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | Bug ID |
| `--resolution <方案>` | 否 | 解决方案：fixed/bydesign/duplicate/postponed/willnotfix/notrepro/... |
| `--resolved-build <build>` | 否 | 解决版本 |
| `--build-name <名称>` | 否 | 新建 Build 名称（与 `--resolved-build` 配合） |
| `--assigned-to <账号>` | 否 | 解决后指派给 |
| `--comment <备注>` | 否 | 附带评论 |

```bash
zentao-cli bug resolve 2001 --resolution fixed --comment "已修复"
```

#### bug activate

激活 Bug（resolved/closed → active）。

```
zentao-cli bug activate <id> [--assigned-to <账号>] [--opened-build <build>] [--comment <备注>]
```

```bash
zentao-cli bug activate 2001 --assigned-to <你的账号> --comment "需要重新处理"
```

#### bug close

关闭 Bug（→ closed）。

```
zentao-cli bug close <id> [--comment <备注>]
```

> 部分账号可能无此权限（透传 `user-deny`）。

```bash
zentao-cli bug close 2001 --comment "客户确认无需修复"
```

#### bug confirm

确认 Bug。

```
zentao-cli bug confirm <id> [--comment <备注>]
```

> 部分账号可能无此权限（透传 `user-deny`）。

```bash
zentao-cli bug confirm 2001 --comment "已确认"
```

#### bug comment

给 Bug 添加评论。

```
zentao-cli bug comment <id> --comment <内容>
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `<id>` | 是 | Bug ID |
| `--comment <内容>` | 是 | 评论内容，不能为空 |

```bash
zentao-cli bug comment 2001 --comment "和产品确认了，下个版本修"
```

## 配置与凭据安全

### 配置文件格式

登录相关参数保存在平台标准用户配置目录（Linux: `~/.config/zentao-cli/config.toml`），用 `zentao-cli config path` 可查到实际路径。完整示例：

```toml
default_profile = "default"

[profiles.default]
server = "https://zentao.example.com"
account = "<你的账号>"
timeout_seconds = 60
password = "<你的密码>"

# 多环境示例：额外的 profile
[profiles.customer]
server = "https://customer.example.com"
account = "<你的账号>"
timeout_seconds = 30
```

| 字段 | 说明 |
|---|---|
| `default_profile` | 默认使用的 profile 名 |
| `profiles.<name>.server` | 禅道服务器地址 |
| `profiles.<name>.account` | 登录账号 |
| `profiles.<name>.timeout_seconds` | 请求超时秒数；0 或不写表示用默认 30 秒 |
| `profiles.<name>.password` | 可选登录密码（明文）；不写则不保存 |

说明：

- `password` 为**可选**字段：显式 `config set password` 才写入；不写则每次登录仍走无回显交互输入。
- 可用 `zentao-cli config set ...` 管理，也可手工编辑后直接使用。
- 多环境用 `--profile <name>` 切换；每个 profile 有独立的 `server`/`account`/`timeout_seconds`/`password`。

### 凭据安全

- 密码**可选**保存到配置：明文存于 `config.toml`，文件权限 `0o600`（仅当前用户可读写）；`config show` 掩码显示，不打印明文。
- 不保存密码时，密码仅在登录时经无回显终端读取；无 TTY（管道/脚本）时退化为 stdin 读取并给出警告。
- Session Cookie 单独保存为 `session-<profile>.json`，权限 `0o600`。
- 会话过期且已保存密码时，查询/写命令自动重登并重试一次；密码始终不进入命令行参数与日志。
- 登录采用旧版表单协议：`verifyRand` + `MD5(MD5(密码)+verifyRand)`，会话只依赖 `zentaosid` Cookie。
- 注意：明文保存密码是可用性与安全的取舍；服务器地址/账号/密码均属敏感信息，请勿把 config.toml 或会话文件提交到公开仓库或分享给他人。

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

- `会话已过期，请重新登录`：重新执行 `login`；若已 `config set password`，查询/写命令会自动重登并重试。
- `请求失败/HTTP 错误`：检查网络、服务器地址和 TLS。
- `远端响应无法解析`：目标实例可能已升级或结构变化，需更新适配层。
- `--verbose` 查看详细日志（日志不包含密码和 Cookie）。

## 卸载

```bash
# 脚本安装
rm -f ~/.local/bin/zentao-cli

# cargo 安装
cargo uninstall zentao-cli

# 配置与会话
rm -rf ~/.config/zentao-cli
```

## 开发

```bash
cargo build
cargo test              # 单元 + wiremock 契约测试 + CLI 端到端测试
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
```

分层结构：

```text
src/cli/            参数解析、输出格式、退出码
src/application/    端口（trait）：认证与三类查询
src/domain/         DTO、枚举、错误模型（不依赖 clap/reqwest）
src/adapters/zentao_v9/  禅道 v9.0.3 旧版 .json 接口唯一适配层
src/infrastructure/ 配置、会话存储、日志
```
