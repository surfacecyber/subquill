# Subquill

[English](README.md)

从 **Bilibili** 与 **YouTube** 视频字幕生成学习笔记的桌面应用。

- **技术栈**：Tauri 2、React、TypeScript、Vite、Rust
- **平台**：macOS + Windows
- **文档**：[文档索引](docs/README.zh-CN.md)

## 为什么做 Subquill

粘贴公开视频链接，拉取字幕（含可用的 AI 字幕），通过兼容 OpenAI 协议的 LLM 生成结构化 Markdown 学习笔记。API Key 与 Cookie 保存后不会在前端明文回显。

## 前置条件

- [Node.js](https://nodejs.org/) LTS
- [Rust](https://www.rust-lang.org/tools/install) stable 工具链
- [Tauri 2](https://v2.tauri.app/start/prerequisites/) 所需的系统依赖

## 快速开始

```bash
npm install
npm run tauri dev
```

Vite 开发服务器监听 **1420** 端口。Tauri 窗口约 **960×700**。

## 功能（v0.1）

- 首次引导：Base URL、模型、API Key、语言、可选 B 站 Cookie（`SESSDATA`）
- 凭据仅落盘；界面只显示「已配置」占位，不回显密钥
- 在引导页与设置中测试 LLM 连接
- 主工作区：粘贴 B 站或 YouTube 链接、预览元数据、生成笔记、进度跟踪、取消任务
- 安全 Markdown 预览（不渲染原始 HTML）、复制、通过系统对话框导出 `.md`
- 可选笔记保存目录：自动将生成的 Markdown 写入所选文件夹
- 设置：更新配置、测试连接、清除已保存的 B 站 Cookie
- 关于：应用名、版本、源码仓库链接
- 界面语言：`zh` / `en` / 跟随系统

**平台说明**

- **Bilibili**：公开字幕与 AI/需登录字幕；可选 `SESSDATA` 解锁部分轨道
- **YouTube**：仅公开字幕；不支持登录 YouTube 账号

## 配置路径

| 文件 | macOS / Linux | Windows |
| --- | --- | --- |
| `settings.json` | `~/.config/subquill/settings.json` | `%APPDATA%\subquill\settings.json` |
| `auth.json` | `~/.local/share/subquill/auth.json` | `%LOCALAPPDATA%\subquill\auth.json` |

切勿将 API Key 或 Cookie 提交到仓库。这些文件位于仓库之外。

## 验证

```bash
npm run typecheck
npm test
cd src-tauri && cargo fmt --check && cargo test && cargo clippy -- -D warnings && cd ..
npm run build
npm run tauri build
```

## 文档

| 文档 | 说明 |
| --- | --- |
| [docs/README.zh-CN.md](docs/README.zh-CN.md) | 文档索引 |
| [实现计划](docs/IMPLEMENTATION_PLAN.zh-CN.md) | 架构、安全边界、模块说明 |
| [发布说明](docs/RELEASING.zh-CN.md) | 版本管理与源码分发 |

面向 AI 工具的仓库指引见英文版 [AGENTS.md](AGENTS.md) / [CLAUDE.md](CLAUDE.md)（未另做中文镜像）。

## 分发说明

Subquill **仅提供源码**，不提供预编译安装包。克隆仓库后本地构建：

```bash
npm install
npm run tauri build
```

不发布 DMG、NSIS、EXE 等安装包。版本管理见 [docs/RELEASING.zh-CN.md](docs/RELEASING.zh-CN.md)。

## 贡献

1. 勿将密钥写入仓库或日志。
2. 网络与凭据处理优先放在 Rust；React 层仅负责 UI。
3. 提交 PR 前运行上方验证命令。
4. 架构与错误码约定见 [docs/IMPLEMENTATION_PLAN.zh-CN.md](docs/IMPLEMENTATION_PLAN.zh-CN.md)。
