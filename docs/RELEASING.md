# Source Distribution and Versioning

[中文](RELEASING.zh-CN.md) · [Docs index](README.md)

Subquill is distributed **source-only**. No pre-built installers (DMG, NSIS, EXE) or in-app binary updates are published.

## Versioning

Keep these files on the same semver (currently `0.1.0`):

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Before creating a source tag:

```bash
npm run check:versions
```

Tags are optional source references. Pushing a tag does not publish installers or trigger a release workflow.

GitHub may expose source archives for repository tags. Subquill does not publish pre-built installers or in-app updates. Pull requests and pushes to `main` run the source quality checks in `.github/workflows/ci.yml`.

## Deterministic checks

Run the same non-network checks used by CI:

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

The live YouTube integration test is excluded from normal test runs because platform availability and bot checks are outside the project's control. To run it explicitly, use `cargo test live_fetch_me_at_the_zoo -- --ignored` from `src-tauri`.

## Building locally

Prerequisites: Node.js LTS, Rust stable, and [Tauri 2 platform dependencies](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev    # development
npm run tauri build  # production build for your OS
```

Users who want Subquill pull the source from GitHub and build on their machine.
