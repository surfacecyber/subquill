# 发布 Subquill

[English](RELEASING.md) · [文档索引](README.zh-CN.md)

本文说明如何为 Subquill 配置 GitHub Actions 发布与 Tauri 2 更新器。

Subquill 区分三层信任：

1. **更新包签名**（必需，不可关闭）— 证明更新归档由发布流水线生成。
2. **macOS 代码签名 / 公证**（内部测试可选）— 降低安装包的 Gatekeeper 警告。
3. **Windows Authenticode**（内部测试可选）— 降低安装包的 SmartScreen 警告。

应用内更新只要求更新器签名密钥。

## 1. 生成更新器密钥对（一次性，仅仓库所有者）

在可信本机执行。**不要在 CI 中生成密钥**，也**切勿提交私钥**。

```bash
npm run tauri signer generate -- -w ~/.subquill/tauri-updater.key
```

命令会打印 **公钥**，并将 **私钥** 写到 `-w` 指定路径。

- 将私钥备份到密码管理器或安全保管库。
- 将公钥存入下方 GitHub Secrets。
- 若丢失私钥，**无法**继续向已信任旧公钥的客户端推送签名更新；需要新密钥对并推动用户手动重装。

## 2. 配置 GitHub Secrets

在 GitHub 仓库设置中添加以下 Secrets：

| Secret | 用途 |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | 更新器私钥（上方生成的文件内容） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 生成私钥时设置的密码（无密码时在 Secret UI 中留空字符串即可） |
| `TAURI_UPDATER_PUBLIC_KEY` | `.pub` 文件中的公钥字符串（单行、无换行） |

`GITHUB_TOKEN` 由平台自动提供。发布工作流仅申请 `contents: write`，用于上传 release 资源。

切勿将私钥粘贴到源码、工作流日志或 issue 评论中。

## 3. 仓库要求

- 仓库必须有 GitHub remote。工作流在运行时根据 `GITHUB_REPOSITORY` 推导更新端点：

  `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`

- `.github/workflows/release.yml` 创建的是 **已发布** release（`releaseDraft: false`，`prerelease: false`），这样 `/releases/latest` 才能正确解析。
- 若只要 **预发布** 内部渠道，不要依赖 `/releases/latest`。改为指向特定版本资源 URL 或自定义端点策略。

## 4. 版本与打标签

保持以下文件 semver 一致（当前为 `0.1.0`）：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

打标签前：

```bash
node scripts/check-versions.mjs
```

通过推送附注标签发布：

```bash
git tag v0.1.0
git push origin v0.1.0
```

`release` 工作流会：

1. 校验版本与标签一致（`vX.Y.Z`）。
2. 生成 gitignore 的 `.release/tauri.release.conf.json`，写入 `pubkey`、更新端点，以及 `bundle.createUpdaterArtifacts=true`。
3. 使用 [`tauri-apps/tauri-action@v1`](https://github.com/tauri-apps/tauri-action) 构建产物（对齐 [Tauri 2 GitHub 流水线文档](https://v2.tauri.app/distribute/pipelines/github/)）：
   - **macOS universal**（`--target universal-apple-darwin --bundles app,dmg`）：更新器用的 `.app.tar.gz` + `.sig`，以及首次安装用的 `.dmg`。
   - **Windows x64**（`--target x86_64-pc-windows-msvc --bundles nsis`）：NSIS `.exe` 安装包及对应更新产物/签名（无 MSI），使 `latest.json` 仅有一个 Windows 目标。
4. 上传安装包、`.sig` 与 `latest.json`。

本地 `tauri.conf.json` 使用 `bundle.targets: ["dmg", "nsis"]` 而非 `all`，避免普通本地构建产出重复的 Windows 包。CI 通过 `--bundles` 显式覆盖如上。

工作流所用 action（对齐 2026-07 Tauri 文档）：`actions/checkout@v7`、`actions/setup-node@v6`、`tauri-apps/tauri-action@v1`、`Swatinem/rust-cache@v2`、`dtolnay/rust-toolchain@stable`。

不带 release 配置的本地构建仍可进行，并跳过更新器配置。

## 5. 本地生成 release 配置（可选）

在本地查看生成的补丁：

```bash
export GITHUB_REPOSITORY=your-org/subquill
export TAURI_UPDATER_PUBLIC_KEY='paste-public-key-here'
node scripts/generate-release-config.mjs
```

脚本会校验输入并写入 `.release/tauri.release.conf.json`，不会打印密钥。

使用补丁构建（macOS 示例 — 与 CI 包集合一致）：

```bash
TAURI_SIGNING_PRIVATE_KEY='…' TAURI_SIGNING_PRIVATE_KEY_PASSWORD='…' \
  npm run tauri build -- \
    --config .release/tauri.release.conf.json \
    --target universal-apple-darwin \
    --bundles app,dmg
```

等效的 Windows 本地发布构建：

```bash
npm run tauri build -- \
  --config .release/tauri.release.conf.json \
  --target x86_64-pc-windows-msvc \
  --bundles nsis
```

## 6. 客户端行为

- 带更新器配置的发布构建会在启动时检查一次；有更新时显示可关闭横幅。
- `App` 中单一更新器控制器在横幅与关于页之间共享状态（不会重复自动检查）。
- 下载/安装前需用户确认；不会静默强制更新。
- 开发构建跳过自动检查；手动检查会提示当前构建未配置更新。
- 关于 → **检查更新** 展示实时状态。

## 7. 未签名安装包（内部测试）

未使用 Apple Developer ID 或 Windows Authenticode 构建的安装包，首次运行可能出现安全提示。这对内部测试是预期行为，与更新包签名无关。
