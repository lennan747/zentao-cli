# 变更日志

本项目遵循[语义化版本](https://semver.org/lang/zh-CN/)。所有值得注意的变更记录于此。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。

## [Unreleased]

### 新增

- 用户查询命令 `zentao-cli user list` / `user search <关键词>`：输出「账号 → 真实姓名」映射（table/json）。数据来自旧版页面响应顶层附带的全量 `users` 映射（主源 `my-task.json`，备源 `project-all-0.json`），普通权限账号可访问。
- 写命令指派人姓名解析：所有 `--assigned-to` 与 `task/bug assign <位置参数>` 支持账号或姓名输入（账号精确 → 姓名精确 → 账号/姓名包含匹配，大小写不敏感）。唯一命中自动采用并在确认摘要显示映射 `输入 → 账号（姓名）`；多候选时 TTY 下编号交互选择、非 TTY 报错并列出候选（退出码 6）；0 命中报错并给相近候选建议；用户列表获取失败时纯 ASCII 输入按账号原样直通（附警告），姓名输入报错（退出码 6）。
- 自更新命令 `zentao-cli update`：查询最新版本、下载发布资产、SHA256 校验、解包并替换当前可执行文件（支持 Linux x86_64 / macOS arm64 / Windows x86_64；默认交互确认，`--yes`/`--dry-run`/`--check`/`--version`/`--repo` 可选；镜像可用 `ZENTAO_CLI_UPDATE_API` / `ZENTAO_CLI_UPDATE_DOWNLOAD` 覆盖）。Windows 下旧文件改名为 `zentao-cli.exe.old`。
- 详情富文本图片提取：`project get` / `task get` / `bug get` 提取 `desc`/`steps` 中的 `<img>` 为绝对 URL 列表（`desc_images` / `steps_images`，无图片时省略），表格输出中图片 URL 内嵌在「描述 / 重现步骤」单元格末尾，且富文本段落换行与原页面一致（不再压成单行）。
- Bug 详情历史记录：`bug get` 解析 `data.actions` 为 `history` 动作日志（`date`/`actor`/`action`/`comment`/`fields`），表格输出本地化为中文（`时间 操作人 动作: 字段 旧值 → 新值`）。
- Bug 详情项目名：`BugDetail.project_name`（来自 `bug.projectName`），表格「所属项目」显示名称而非 ID。

### 变更

- 写命令指派人参数的空白输入（空串/纯空格）统一视为「未提供」，不再向表单提交空值；`task/bug assign` 的位置参数为空时直接报错（退出码 6）。
- 详情表格字段按固定顺序渲染：Bug 为 `ID → 状态 → 产品 → 所属项目 → 标题 → 重现步骤 → 创建者 → 创建日期 → 历史记录`（隐藏 指派给/优先级/所属产品/严重程度）；项目/任务按逻辑顺序统一（JSON 输出字段与顺序不受影响）。

## [0.1.1] - 2026-08-28

### 新增

- 一键安装支持 Windows：`install.ps1`（PowerShell 5.1+，x86_64），`irm ... | iex` 一行安装到 `%LOCALAPPDATA%\zentao-cli\bin` 并自动加入用户 PATH；强制 SHA256 校验。
- 配置可选保存密码（`config set password`，明文存 config.toml，权限 0o600，`config show` 掩码）；`login` 已存密码时免交互。
- 会话过期自动恢复：`project/task/bug` 遇退出码 3 且已保存密码时自动重登并重试一次。
- 面向 AI Agent 的安装指南 `docs/install-guide.md`（双语）与 README 一句话安装。

### 修复

- 发布资产 `SHA256SUMS` 现包含全部平台三行（修复多 job 同名上传互相覆盖，仅余一行的问题）；install.sh 在 macOS 的校验因此恢复正常。
- install.sh 在 macOS Intel 上提前明确报错（未提供该架构预编译二进制）。

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

[Unreleased]: https://github.com/lennan747/zentao-cli/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/lennan747/zentao-cli/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/lennan747/zentao-cli/releases/tag/v0.1.0
