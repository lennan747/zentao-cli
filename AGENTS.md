# zentao-cli 项目约定

## 项目边界

- 本项目是独立的 Rust CLI 应用，位于 `code/zentao-cli/`。
- 只与禅道v9.0.3 的旧版 `.json` 接口交互，首期只读。

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

- 不保存密码到配置文件或会话文件。
- Cookie/Session 文件权限为 `0o600`。
- `.env` 凭据只在开发/探索阶段临时使用，不进入仓库。

## 测试

- 单元测试：`cargo test`
- 静态检查：`cargo clippy -- -D warnings`
- 真实环境冒烟：使用 release 二进制 + 本地 `.env` 凭据，只读。
