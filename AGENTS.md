# zentao-cli 项目约定

## 项目边界

- 本项目是独立的 Rust CLI 应用，位于项目根目录。
- 只与禅道v9.0.3 的旧版 `.json` 接口交互，首期只读。
- 用户列表（账号→真实姓名）来自旧版页面响应顶层附带的 `users` 映射：主源 `my-task.json`、备源 `project-all-0.json`（`ZentaoV9UserGateway` 内做降级链）；`user-index.json` 仅管理员可访问，不使用。
- `update` 命令只访问 GitHub Releases（默认 `lennan747/zentao-cli`），可用 `ZENTAO_CLI_UPDATE_API` / `ZENTAO_CLI_UPDATE_DOWNLOAD` 覆盖镜像地址。

## 分层依赖

```text
cli/
    ↓
application/
    ↓
domain/
    ↑
adapters/zentao_v9/
    ↑
infrastructure/
```

- `domain` 不依赖 `clap`、`reqwest`、文件系统或具体路由。
- `adapters/zentao_v9` 是唯一知道旧版路由和响应结构的模块。
- `cli` 只负责参数转换、调用用例、选择 formatter 和退出码。

## 安全

- 密码**可选**保存到配置文件（`config set password`，明文）；不保存到会话文件。
- 配置文件与 Cookie/Session 文件权限均为 `0o600`；`config show` 密码掩码，不打印明文。
- 密码不进入命令行参数与日志。
- `.env` 凭据只在开发/探索阶段临时使用，不进入仓库。

## 测试

- 单元测试：`cargo test`
- 静态检查：`cargo clippy -- -D warnings`
- 真实环境冒烟：使用 release 二进制 + 本地 `.env` 凭据，只读。
