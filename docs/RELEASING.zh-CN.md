# 源码分发与版本管理

[English](RELEASING.md) · [文档索引](README.zh-CN.md)

Subquill **仅以源码分发**。不发布预编译安装包（DMG、NSIS、EXE），也不提供应用内二进制更新。

## 版本号

以下文件保持相同 semver（当前 `0.1.0`）：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

创建源码 tag 前：

```bash
npm run check:versions
```

Tag 仅作为可选源码版本引用。推送 tag 不会发布安装包，也不会触发发布工作流。

GitHub 可能会为仓库 tag 提供源码归档。Subquill 不发布预编译安装包，也不提供应用内更新。针对 `main` 的 PR 和 push 会运行 `.github/workflows/ci.yml` 中的源码质量检查。

## 确定性检查

运行与 CI 相同的非联网检查：

```bash
npm ci
npm run typecheck
npm test
npm run build
npm run check:versions
cd src-tauri
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -D warnings
```

常规测试不会运行实时 YouTube 集成测试，因为平台可用性和反机器人检查不受项目控制。需要手动测试时，在 `src-tauri` 目录运行 `cargo test live_fetch_me_at_the_zoo -- --ignored`。

## 本地构建

前置条件：Node.js LTS、Rust stable，以及 [Tauri 2 系统依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
npm install
npm run tauri dev    # 开发
npm run tauri build  # 本机生产构建
```

使用者从 GitHub 拉取源码后在本机构建即可。
