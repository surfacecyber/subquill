# OpenNote

Desktop app for generating study notes from Bilibili / YouTube videos.

- **Stack**: Tauri 2, React, TypeScript, Vite, Rust
- **Platform**: macOS + Windows
- **Docs**: [Implementation plan](docs/IMPLEMENTATION_PLAN.md)

## Local development

```bash
npm install
npm run tauri dev
```

The dev server runs on port 1420. The Tauri window opens at about 960×700.

## Current features (v0.1)

- First-run onboarding: base URL, model, API Key, locale, optional Bilibili cookie
- Credentials are stored on disk only; the UI shows “configured” placeholders and never echoes secrets
- Test LLM connection from onboarding and settings
- Main workspace: paste a Bilibili or YouTube URL, generate notes, track progress, cancel in-flight jobs
- Safe Markdown preview (no raw HTML), copy to clipboard, export to `.md` via native save dialog
- Settings: update configuration, test connection, explicitly clear saved Bilibili cookie
- Optional notes save folder: auto-write generated Markdown into a chosen directory
- About: app name, version, manual update checks with live status
- Chinese and English UI (`zh` / `en` / follow system)

## Configuration paths

| File | macOS / Linux | Windows |
| --- | --- | --- |
| `settings.json` | `~/.config/opennote/settings.json` | `%APPDATA%\opennote\settings.json` |
| `auth.json` | `~/.local/share/opennote/auth.json` | `%LOCALAPPDATA%\opennote\auth.json` |

Never commit API keys or cookies. These files live outside the repository.

## Verification

```bash
npm run typecheck
npm test
cd src-tauri && cargo fmt --check && cargo test && cargo clippy -- -D warnings && cd ..
npm run build
npm run tauri build
```

## Distribution notes

- **Code signing**: Installers are not signed for internal beta builds. macOS Gatekeeper and Windows SmartScreen may warn on first open.
- **Auto-update**: Updater code is integrated. Published release builds need a GitHub remote, repository secrets, and a non-draft `/releases/latest` release. See [docs/RELEASING.md](docs/RELEASING.md).
- **Local builds**: `npm run tauri build` works without updater secrets; the app reports that auto-update is not configured.
