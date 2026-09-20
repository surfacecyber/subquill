# Releasing Subquill

[中文](RELEASING.zh-CN.md) · [Docs index](README.md)

Subquill is distributed **source-only**. No pre-built installers (DMG, NSIS, EXE) or in-app binary updates are published.

## Versioning

Keep these files on the same semver (currently `0.1.0`):

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Before tagging:

```bash
node scripts/check-versions.mjs
```

Publish a release by pushing an annotated tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub creates a release with source archives automatically. The `release` workflow only verifies that project versions match the tag — it does not build or upload installer artifacts.

## Building locally

Prerequisites: Node.js LTS, Rust stable, and [Tauri 2 platform dependencies](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev    # development
npm run tauri build  # production build for your OS
```

Users who want Subquill pull the source from GitHub and build on their machine.
