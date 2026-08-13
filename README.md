# Subquill

[中文](README.zh-CN.md)

Desktop app that generates study notes from **Bilibili** and **YouTube** video subtitles.

- **Stack**: Tauri 2, React, TypeScript, Vite, Rust
- **Platform**: macOS + Windows
- **Docs**: [Documentation index](docs/README.md)

## Why Subquill

Paste a public video URL, fetch captions (including AI captions where available), and get structured Markdown study notes via an OpenAI-compatible LLM — without sending API keys or cookies to the frontend UI after save.

## Prerequisites

- [Node.js](https://nodejs.org/) LTS
- [Rust](https://www.rust-lang.org/tools/install) stable toolchain
- Platform dependencies for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

## Quick start

```bash
npm install
npm run tauri dev
```

The Vite dev server listens on port **1420**. The Tauri window opens at about **960×700**.

## Features (v0.1)

- First-run onboarding: base URL, model, API key, locale, optional Bilibili cookie (`SESSDATA`)
- Credentials on disk only; UI shows “configured” placeholders and never echoes secrets
- Test LLM connection from onboarding and settings
- Main workspace: paste a Bilibili or YouTube URL, preview metadata, generate notes, track progress, cancel jobs
- Safe Markdown preview (no raw HTML), copy to clipboard, export `.md` via native save dialog
- Optional notes save folder: auto-write generated Markdown into a chosen directory
- Settings: update configuration, test connection, clear saved Bilibili cookie
- About: app name, version, manual update checks with live status
- UI locales: `zh` / `en` / follow system

**Platform notes**

- **Bilibili**: public and AI/login-gated captions; optional `SESSDATA` unlocks some tracks
- **YouTube**: public captions only; no YouTube account login

## Configuration paths

| File | macOS / Linux | Windows |
| --- | --- | --- |
| `settings.json` | `~/.config/subquill/settings.json` | `%APPDATA%\subquill\settings.json` |
| `auth.json` | `~/.local/share/subquill/auth.json` | `%LOCALAPPDATA%\subquill\auth.json` |

Never commit API keys or cookies. These files live outside the repository.

## Verification

```bash
npm run typecheck
npm test
cd src-tauri && cargo fmt --check && cargo test && cargo clippy -- -D warnings && cd ..
npm run build
npm run tauri build
```

## Documentation

| Document | Description |
| --- | --- |
| [docs/README.md](docs/README.md) | Documentation index |
| [Implementation plan](docs/IMPLEMENTATION_PLAN.md) | Architecture, security, modules |
| [Releasing](docs/RELEASING.md) | GitHub Actions releases and Tauri updater |

Agent guidance for AI tools: [AGENTS.md](AGENTS.md) / [CLAUDE.md](CLAUDE.md).

## Distribution

- **Code signing**: Installers are not signed for internal beta. macOS Gatekeeper and Windows SmartScreen may warn on first open.
- **Auto-update**: Updater code is integrated. Published release builds need a GitHub remote, repository secrets, and a non-draft `/releases/latest` release. See [docs/RELEASING.md](docs/RELEASING.md).
- **Local builds**: `npm run tauri build` works without updater secrets; the app reports that auto-update is not configured.

## Contributing

1. Keep secrets out of the repo and logs.
2. Prefer Rust for network and credential handling; keep the React layer UI-only.
3. Run the verification commands above before opening a PR.
4. Read [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for architecture and error-code conventions.
