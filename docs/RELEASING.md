# Releasing OpenNote

This document describes how to configure GitHub Actions releases and the Tauri 2 updater for OpenNote.

OpenNote separates three different trust layers:

1. **Updater payload signing** (required, cannot be disabled) — proves update archives were produced by the release pipeline.
2. **macOS code signing / notarization** (optional for internal beta) — reduces Gatekeeper warnings for the installer.
3. **Windows Authenticode** (optional for internal beta) — reduces SmartScreen warnings for the installer.

Only the updater signing keys are required for in-app updates.

## 1. Generate the updater keypair (one-time, owner only)

Run this locally on a trusted machine. **Do not run key generation in CI** and **never commit the private key**.

```bash
npm run tauri signer generate -- -w ~/.opennote/tauri-updater.key
```

The command prints a **public key** and writes the **private key** to the path you pass with `-w`.

- Back up the private key file in a password manager or secure vault.
- Store the public key in GitHub Secrets (below).
- If you lose the private key, you **cannot** ship signed updates to clients that already trust the old public key. You would need a new keypair and a manual reinstall campaign.

## 2. Configure GitHub Secrets

In the GitHub repository settings, add these repository secrets:

| Secret | Purpose |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Private updater signing key (file contents or path content generated above) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the private key, if you set one during generation (leave empty string in the secret UI only if the key has no password) |
| `TAURI_UPDATER_PUBLIC_KEY` | Public key string from the `.pub` file (single line, no newlines) |

`GITHUB_TOKEN` is provided automatically. The release workflow requests only `contents: write` so it can publish release assets.

Never paste private keys into source code, workflow logs, or issue comments.

## 3. Repository requirements

- The repository must have a GitHub remote. Workflows derive the updater endpoint at runtime from `GITHUB_REPOSITORY`:

  `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`

- Releases created by `.github/workflows/release.yml` are **published** (`releaseDraft: false`, `prerelease: false`) so `/releases/latest` resolves correctly.
- If you need **prerelease-only** internal channels, do not rely on `/releases/latest`. Point the generated release config at a version-specific asset URL or a custom endpoint strategy instead.

## 4. Versioning and tagging

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

The `release` workflow will:

1. Verify versions match the tag (`vX.Y.Z`).
2. Generate a gitignored `.release/tauri.release.conf.json` with `pubkey`, updater endpoint, and `bundle.createUpdaterArtifacts=true`.
3. Build release artifacts via [`tauri-apps/tauri-action@v1`](https://github.com/tauri-apps/tauri-action) (aligned with the [Tauri 2 GitHub pipeline docs](https://v2.tauri.app/distribute/pipelines/github/)):
   - **macOS universal** (`--target universal-apple-darwin --bundles app,dmg`): `.app.tar.gz` + `.sig` for the updater, plus a `.dmg` installer for first-time installs.
   - **Windows x64** (`--target x86_64-pc-windows-msvc --bundles nsis`): NSIS `.exe` installer and matching updater artifact/signature only (no MSI) so `latest.json` has a single Windows target.
4. Upload installers, `.sig` files, and `latest.json`.

Local `tauri.conf.json` uses `bundle.targets: ["dmg", "nsis"]` instead of `all`, so ordinary local builds do not produce duplicate Windows bundles. CI overrides bundles explicitly with `--bundles` as above.

Workflow actions (2026-07 Tauri docs): `actions/checkout@v7`, `actions/setup-node@v6`, `tauri-apps/tauri-action@v1`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`.

Local builds without the release config continue to work and skip updater configuration.

## 5. Local release-config generation (optional)

To inspect the generated patch locally:

```bash
export GITHUB_REPOSITORY=your-org/opennote
export TAURI_UPDATER_PUBLIC_KEY='paste-public-key-here'
node scripts/generate-release-config.mjs
```

The script validates input and writes `.release/tauri.release.conf.json`. It never prints secrets.

Build with the patch (macOS example — matches CI bundle set):

```bash
TAURI_SIGNING_PRIVATE_KEY='…' TAURI_SIGNING_PRIVATE_KEY_PASSWORD='…' \
  npm run tauri build -- \
    --config .release/tauri.release.conf.json \
    --target universal-apple-darwin \
    --bundles app,dmg
```

Windows release-equivalent local build:

```bash
npm run tauri build -- \
  --config .release/tauri.release.conf.json \
  --target x86_64-pc-windows-msvc \
  --bundles nsis
```

## 6. Client behavior

- Release builds with updater config check once at startup and show a dismissible banner when an update is available.
- A single updater controller in `App` shares state between the banner and About page (no duplicate auto-checks).
- Users must confirm before download/install; updates are not silently forced.
- Development builds skip automatic checks; manual checks report that the current build is not configured.
- About → **Check for updates** exposes the live status.

## 7. Unsigned installers (internal beta)

Installers built without Apple Developer ID or Windows Authenticode may show first-run security prompts. That is expected for internal testing and is separate from updater payload signing.
