# 发布 Subquill

[English](RELEASING.md) · [文档索引](README.zh-CN.md)

Subquill **仅以源码分发**。不发布预编译安装包（DMG、NSIS、EXE），也不提供应用内二进制更新。

## 版本号

以下文件保持相同 semver（当前 `0.1.0`）：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

打标签前：

```bash
node scripts/check-versions.mjs
```

推送 annotated tag 发布：

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub 会自动为 tag 创建 release 并提供源码归档。`release` 工作流仅校验项目版本与 tag 一致，不构建或上传安装包产物。

## 本地构建

前置条件：Node.js LTS、Rust stable，以及 [Tauri 2 系统依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
npm install
npm run tauri dev    # 开发
npm run tauri build  # 本机生产构建
```

使用者从 GitHub 拉取源码后在本机构建即可。
