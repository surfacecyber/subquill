# Subquill Implementation Plan

[中文](IMPLEMENTATION_PLAN.zh-CN.md) · [Docs index](README.md)

This document is the canonical engineering plan for Subquill: a macOS + Windows desktop app (Tauri 2 + React + TypeScript + Vite) that generates study notes from **Bilibili** and **YouTube** video subtitles. Business logic lives in Rust; Tauri commands are a thin IPC adapter.

## Architecture principles

| Layer | Responsibility |
| --- | --- |
| `src/` (React) | UI, i18n, form validation UX only — no secrets in `localStorage` |
| `src-tauri/src/commands/` | Thin `#[tauri::command]` wrappers |
| `src-tauri/src/settings/` | Versioned non-secret config (`settings.json`) |
| `src-tauri/src/auth/` | Versioned secrets (`auth.json`), status-only IPC |
| `media/` | Route Bilibili / YouTube URLs; unified preview + subtitle fetch — **implemented** |
| `bilibili/` | URL parse, view API, subtitle fetch — **implemented** |
| `youtube/` | URL parse, InnerTube player API, public captions — **implemented** |
| `llm/` | OpenAI-compatible client, JSON validation — **implemented** |
| `note/` | In-memory note assembly, chunking, Markdown — **implemented** |
| `job/` | `job_id`, cancel, timeout, progress events — **implemented** |

Do **not** add empty Rust modules that compile but do nothing.

## Repository layout

Single application root (not a monorepo):

```
subquill/
  src/                 # React UI
  src-tauri/           # Rust backend + Tauri shell
  docs/
```

## Local configuration

### Paths

| File | macOS / Linux | Windows |
| --- | --- | --- |
| `settings.json` | `~/.config/subquill/settings.json` (`$XDG_CONFIG_HOME/subquill`) | `%APPDATA%\subquill\settings.json` |
| `auth.json` | `~/.local/share/subquill/auth.json` (`$XDG_DATA_HOME/subquill`) | `%LOCALAPPDATA%\subquill\auth.json` |

### `settings.json` (version 1)

Non-secret, safe to log field names (not values in production):

- `version`
- `base_url` — OpenAI-compatible API base; `http://` / `https://` only (localhost allowed for local models)
- `model`
- `locale` — `zh` | `en` | `system`
- `onboarding_completed`
- `notes_save_dir` — optional absolute path; when set, completed notes are auto-written as Markdown files

**Never** store `api_key`, cookies, or tokens in `settings.json`.

### `auth.json` (version 1)

Secrets only:

- `version`
- `api_key` — OpenAI-compatible key
- `bilibili_cookie` — optional; required only when Bilibili APIs need login

IPC rule: frontend receives **booleans only** via `get_auth_status` (`has_api_key`, `has_bilibili_cookie`). Plaintext secrets are never returned.

### Atomic writes & permissions

1. Ensure parent directory exists.
2. Write `.<name>.tmp` in the same directory.
3. Unix: directory `0700`, file `0600`.
4. `rename` temp → final path.
5. Errors must not echo secret values.

## Security boundaries

### Cookie / credential boundary

- Bilibili cookie stays in `auth.json` and Rust memory after load.
- Only Rust calls Bilibili HTTP APIs.
- Cookie is sent **only** to `https://api.bilibili.com` view/player endpoints — never to `b23.tv` short-link hops or subtitle CDN hosts.
- Frontend may collect cookie once on onboarding; after save, UI shows placeholders only.
- Do not log cookie or API key values.

### Bilibili URL & CDN allowlists

- Video page hosts (exact): `www.bilibili.com`, `m.bilibili.com`.
- Short-link host (exact): `b23.tv` — resolved manually with per-hop host validation, no cookie, redirect cap.
- API host (exact): `api.bilibili.com`.
- Subtitle CDN hosts (suffix): `hdslb.com`, `bilibili.com`, `bilivideo.com` — suffix match only (blocks `evil-hdslb.com`, `hdslb.com.evil`).
- Reject URLs with embedded credentials, non-http(s) schemes, and BV/av ids parsed from fragments only.

### Markdown as untrusted input

LLM output and fetched descriptions are rendered as Markdown. Treat all Markdown as **untrusted**:

- Sanitize or use a safe renderer (no raw `dangerouslySetInnerHTML` without a vetted pipeline).
- Block `javascript:`, event handlers, and remote resource loads in rendered HTML.
- CSP must not allow arbitrary remote scripts (see `tauri.conf.json`).

### Tauri capabilities

- Minimal `core:default` only for the main window.
- No broad `shell`, `fs`, or `http` plugin scopes unless a feature needs them.
- Prefer Rust-side HTTP (`reqwest` in `bilibili/` / `youtube/` / `llm/`) over giving the webview network access to third parties.

### Code signing vs updater signing

These are **different** concerns:

| Concern | Purpose | When |
| --- | --- | --- |
| **OS code signing** (Apple Developer ID + notarization, Windows Authenticode) | Users can open the installer/app without Gatekeeper/SmartScreen warnings | Before public release |
| **Tauri updater signing** (minisign key pair, public key in `tauri.conf.json`) | Verify update payloads were published by us | Before enabling auto-update |

Internal beta may skip OS code signing. **Updater signing is mandatory before shipping auto-updates**, even if OS signing is deferred. Never commit private keys; use CI secrets.

## Structured errors

All command failures return:

```json
{ "code": "VALIDATION_ERROR", "message": "human-readable, no secrets" }
```

Standard codes:

| Code | Meaning |
| --- | --- |
| `VALIDATION_ERROR` | Input failed trim/format checks |
| `STORAGE_ERROR` | IO / permissions / atomic write failure |
| `SERIALIZATION_ERROR` | Corrupt or unsupported JSON version |
| `NETWORK_ERROR` | HTTP transport failures |
| `SHORT_LINK_FAILED` | b23.tv resolution failed |
| `VIDEO_NOT_FOUND` | View / player API: video missing |
| `VIDEO_RESTRICTED` | YouTube (or similar): login / age / region restriction; app does not support YouTube accounts |
| `API_REJECTED` | Platform API returned a rejected / non-zero response |
| `PART_OUT_OF_RANGE` | Requested `p` exceeds page count |
| `AUTH_REQUIRED` | Login cookie likely required (e.g. missing `subtitle_url`) |
| `RATE_LIMITED` | HTTP 412 / 429 from Bilibili |
| `NO_SUBTITLE` | No subtitle tracks available |
| `SUBTITLE_CORRUPT` | Subtitle JSON invalid or empty after parse |
| `LLM_AUTH_ERROR` | LLM API rejected credentials |
| `LLM_RATE_LIMITED` | LLM API rate limit (HTTP 429) |
| `LLM_TIMEOUT` | LLM request timed out |
| `LLM_NETWORK_ERROR` | LLM HTTP transport failure |
| `LLM_API_ERROR` | LLM API returned an error response |
| `LLM_INVALID_RESPONSE` | LLM output failed JSON or semantic validation |
| `JOB_CANCELLED` | User cancelled job |
| `JOB_TIMEOUT` | Job exceeded deadline |
| `JOB_ALREADY_RUNNING` | A note job is already active |
| `JOB_NOT_FOUND` | Unknown `job_id` |
| `JOB_NOT_READY` | Job is still queued or running |
| `INTERNAL_ERROR` | Unexpected internal failure (poisoned lock, task panic) |

Frontend maps codes to i18n strings; never surface raw Rust backtraces.

## Job model (`job/`)

**Status: implemented (v1 in-memory orchestration).**

- Every long operation gets a `job_id` (UUID).
- States: `queued` → `running` → `completed` | `failed` | `cancelled`.
- **Single active job**: starting a new job while another is `queued` or `running` returns `JOB_ALREADY_RUNNING`. Terminal jobs are replaced when a new job starts.
- Commands: `start_note_job`, `cancel_job`, `get_job_status`, `get_job_result`.
- `start_note_job` returns `{ job_id }` immediately; an outer async task awaits `spawn_blocking`. Join failures mark the job `failed` with `INTERNAL_ERROR`.
- Progress events: `job://progress` with structured `JobProgress` (stage, optional chunk index/count, optional `error_code` on terminal stages — no message/secrets). **Terminal state is written in the manager before the event is emitted** so `get_job_result` is consistent with progress events.
- `get_job_result`: returns Markdown result when `completed`; returns stored `ErrorPayload` when `failed`; returns `JOB_CANCELLED` (or stored error) when `cancelled`; only `queued`/`running` return `JOB_NOT_READY`.
- `cancel_job`: sets the cancel token and returns `cancel_requested: true` while status may still be `running` until the background task observes cancellation.
- Job deadline: 30 minutes wall-clock (`JOB_DEADLINE`). Per-request HTTP timeouts remain unchanged.
- **Cancellation semantics**: cancel sets an in-memory flag checked after subtitle fetch, before/after each LLM chunk, after render, and atomically inside `JobManager::complete` (cancel wins over late success). Blocking HTTP cannot be interrupted mid-request; results from in-flight requests are discarded once they return if cancel was requested.
- Finished result stores final Markdown, title, video id / bvid, language, `segment_count` — not full intermediate LLM responses.
- All state in Rust memory only (no DB in v1); app exit clears jobs.

## Media routing (`media/`)

**Status: implemented.**

- Detect YouTube vs Bilibili from the URL host / shape.
- `preview_video` and `fetch_subtitles` share one entry point; cookie is passed only into the Bilibili path.
- YouTube results are adapted into the same subtitle / preview shapes the note pipeline already consumes.
- Workspace UI calls `preview_video` / job start; legacy `fetch_bilibili_subtitles` remains as a thin command for Bilibili-only fetch.

## Bilibili integration (`bilibili/`)

**Status: implemented (v1 subtitle fetch).**

- Parse BV/av URLs and `p` part index in Rust (`url.rs`).
- Resolve `b23.tv` short links with manual redirects and host allowlist.
- **Title source**: Bilibili **view API** (`x/web-interface/view`).
- Subtitles: `x/player/wbi/v2` (`bvid`+`cid`, same as BiliNote) → pick track (manual zh > AI zh > any zh > first valid) → download JSON from allowlisted CDN. Do **not** use `/x/player/v2` with `aid` — it can return empty or wrong-video `subtitle_url`.
- Cookie from `auth` only on `api.bilibili.com` requests.
- HTTP responses are bounded before read: API/redirect ~2 MiB / 64 KiB, subtitle CDN up to 16 MiB; oversize bodies fail without echoing response content.
- Tauri command: `fetch_bilibili_subtitles` (thin wrapper in `commands/`).
- Unit + mock HTTP tests; no real network in CI.

**Known risks / deferred**

- **Player API**: v1 calls `/x/player/wbi/v2` with `bvid`+`cid` (aligned with BiliNote). Avoid `/x/player/v2?aid=…` which may return empty or mismatched AI subtitle URLs. Empty lists / empty URLs are retried once.
- Rate limits / ToS: see release checklist.

## YouTube integration (`youtube/`)

**Status: implemented (v1 public captions).**

- Parse `youtube.com` / `youtu.be` / `m.youtube.com` / `music.youtube.com` (and `*.youtube.com`) watch, shorts, and short-link URLs.
- Fetch metadata and caption tracks via the public InnerTube Android player endpoint (`youtubei/v1/player`); no YouTube account cookie.
- Prefer usable caption tracks; download timedtext / JSON3 from allowlisted hosts.
- Restricted / age-gated / login-required videos surface `VIDEO_RESTRICTED` — Subquill does **not** support YouTube sign-in.
- Unit tests cover URL parsing and selection; live network fetch tests are optional / ignored in normal CI.

## LLM integration (`llm/`)

**Status: implemented (v1 chat/completions client).**

- OpenAI-compatible `chat/completions` using `base_url` (API v1 root), `api_key`, `model`.
- `base_url` is joined safely to `chat/completions` (no `/v1/v1` duplication).
- Prompt-only JSON (no `response_format` / `json_schema` dependency).
- Blocking `reqwest` + rustls; connect/total timeouts; response body size cap; injectable HTTP transport for tests.
- Errors: `LLM_AUTH_ERROR`, `LLM_RATE_LIMITED`, `LLM_TIMEOUT`, `LLM_NETWORK_ERROR`, `LLM_API_ERROR`, `LLM_INVALID_RESPONSE`.
- Tauri command: `test_llm` (thin wrapper in `commands/`).

## Note pipeline (`note/`)

**Status: implemented (v1 in-memory assembly).**

- **Internal JSON uses integer milliseconds** (`start_ms`, `end_ms`) for validation and merging; human-readable `HH:MM:SS` (including >1h) is applied only in prompts and final Markdown.
- `chunk_segments`: conservative char budget, complete segment boundaries; never drops or reorders segments.
- Per-chunk LLM JSON: `{summary, sections:[{start_ms,end_ms,title,explanation}]}` with serde + semantic validation and one constrained retry.
- Multi-chunk: rule-based section merge/sort/dedup; optional summary merge LLM call when multiple chunks.
- `render_markdown`: `# title`, summary, section breakdown, full transcript from **original** segments (not LLM-rewritten).
- Markdown/HTML escaping on untrusted titles and LLM text.
- Service API: `generate_note_data(client, metadata, segments, locale, cancellation_check, progress)` — no Tauri coupling.
- No DB / history in v1.

## yt-dlp

**Deferred.** Prefer official/subtitle APIs first. If yt-dlp is added later:

- Bundle or invoke as a sidecar with pinned version.
- Strict argument allowlist; never pass user-controlled shell fragments.
- Separate capability review (subprocess, temp files).

## Testing strategy

| Area | Approach |
| --- | --- |
| Paths | Pure functions + env overrides (`XDG_*`, temp dirs) |
| Validation | Unit tests for URL, locale, model |
| Storage | `tempfile` round-trip for atomic write |
| Auth/settings separation | Assert serialized JSON field sets |
| Frontend | Vitest for i18n and pure TS helpers |
| Bilibili | URL parse, host allowlist, track priority, mock HTTP integration |
| YouTube | URL parse / host checks, track selection, restricted-video mapping |
| Media | Platform routing (Bilibili vs YouTube) |
| LLM / note | Endpoint join, error mapping, chunk boundaries, validation, merge, Markdown escape, mock e2e |
| Job | State machine, single-active enforcement, cancel idempotency, deadline/cancel mapping, progress payload safety, mock pipeline orchestration |
| Integration | `tauri dev` manual; no real network in CI unit tests |

## Release checklist (later)

See also [RELEASING.md](RELEASING.md) for the shipping pipeline.

- [ ] OS code signing (macOS + Windows)
- [ ] Tauri updater key pair in CI
- [ ] CSP review (Markdown renderer is in place; re-audit if HTML pipeline changes)
- [ ] Secret redaction audit on logs
- [ ] Bilibili / YouTube ToS and rate-limit review
