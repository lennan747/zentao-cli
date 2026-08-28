# zentao-cli 安装指南（面向 AI Agent）
# zentao-cli Install Guide (for AI Agents)

本指南面向 AI Agent（Claude Code、opencode、Cursor、Trae 等）。安装过程中需要用户参与的步骤已明确标注。
This guide is designed for AI Agents (Claude Code, opencode, Cursor, Trae, etc.). Steps that require user interaction are marked explicitly.

## 前置条件 / Prerequisites

- Linux/macOS：`curl` 与 `sh`；Windows：PowerShell 5.1+（x86_64）。若从源码构建需 Rust ≥ 1.80。
- Linux/macOS: `curl` and `sh`; Windows: PowerShell 5.1+ (x86_64). Rust ≥ 1.80 if building from source.

## Step 1 — 安装 CLI / Install CLI

Linux / macOS（arm64）：

```shell
curl -fsSL https://raw.githubusercontent.com/lennan747/zentao-cli/master/install.sh | sh
```

Windows（PowerShell 5.1+，x86_64）：

```powershell
irm https://raw.githubusercontent.com/lennan747/zentao-cli/master/install.ps1 | iex
```

> Windows 默认装到 `%LOCALAPPDATA%\zentao-cli\bin` 并自动加入用户 PATH，**新开终端生效**。AI 安装后如需当前会话直接调用，用完整路径 `%LOCALAPPDATA%\zentao-cli\bin\zentao-cli.exe`。

安装后确认版本：

```shell
zentao-cli --version   # 期望输出 zentao-cli 0.1.x
```

## Step 2 — 安装 Skill / Install Skill

选择目标 AI Agent 对应的方式之一即可。

- **Claude Code**：

  ```shell
  npx -y skills add lennan747/zentao-cli -y -g
  ```

- **opencode**（复制或软链到全局技能目录，**改完需重启 opencode**）：

  ```shell
  mkdir -p ~/.config/opencode/skills
  cp -r <仓库>/skills/zentao-cli ~/.config/opencode/skills/
  ```

  或在 `~/.config/opencode/opencode.json` 注册路径，免复制：

  ```json
  { "skills": { "paths": ["<仓库>/skills"] } }
  ```

- **Cursor / Trae / 其他支持 Agent Skills 的 Agent**：把 `skills/zentao-cli/SKILL.md` 复制到对应 skills 目录；不支持 skill 目录的 Agent 则直接读取该 `SKILL.md` 作为操作手册。

Skill 源文件：<https://raw.githubusercontent.com/lennan747/zentao-cli/master/skills/zentao-cli/SKILL.md>

## Step 3 — 配置 / Configure

向用户索取禅道服务器地址与登录账号，然后：

```shell
zentao-cli config set server <服务器地址>
zentao-cli config set account <登录账号>
```

> 服务器地址/账号属于用户敏感信息，仅写入本地配置，不要输出到公开渠道。

## Step 4 — 登录 / Login（可能需要用户参与 / may require user）

```shell
zentao-cli login
```

两种方式，任选其一：

- **推荐（免交互，适合 AI 自动重登）**：向用户索取密码后写入配置，此后 `login` 与「会话过期自动重登」都无需用户再输入：

  ```shell
  zentao-cli config set password <密码>   # 明文存 config.toml，权限 0o600，config show 掩码显示
  zentao-cli login                        # 使用已保存密码，免输入
  ```

- **交互式**：未保存密码时，`login` 用无回显终端读取密码（`rpassword`），**AI 无法代输**——必须把终端交给用户输入；无 TTY 时退化为 stdin 读取。
  If no password is saved, login reads it via a no-echo prompt. The AI cannot type it — hand the terminal to the user.

## Step 5 — 验证 / Verify

```shell
zentao-cli project list
```

退出码 `0` 即安装成功。查询/写命令遇会话过期（退出码 `3`）且已保存密码时，会**自动重登并重试一次**；未保存密码则重新执行 Step 4。

## 安全须知 / Security

- 不得把服务器地址、账号、密码、任务/Bug 内容、会话信息输出到公开渠道或写入仓库提交。
- 密码**可选**保存到配置（明文，config.toml 权限 `0o600`，`config show` 掩码）；未保存则仅登录时交互输入。会话文件权限 `0o600`。密码不进入命令行参数与日志。
- 写操作默认交互确认；经用户明确同意前不得使用 `--yes`。

## 退出码参考 / Exit Codes

| 退出码 | 含义 |
|---|---|
| 0 | 成功 |
| 3 | 认证/会话过期/未登录 |
| 4 | 资源不存在/无权限 |
| 6 | 参数无效/远端拒绝/网络错误 |
| 7 | 内部错误 |

更多命令与使用准则见 `skills/zentao-cli/SKILL.md`。
