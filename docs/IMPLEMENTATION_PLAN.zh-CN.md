# OpenNote 实现计划

[English](IMPLEMENTATION_PLAN.md) · [文档索引](README.zh-CN.md)

本文是 OpenNote 的权威工程计划：面向 macOS + Windows 的桌面应用（Tauri 2 + React + TypeScript + Vite），从 **Bilibili** 与 **YouTube** 视频字幕生成学习笔记。业务逻辑在 Rust 中实现；Tauri command 仅作薄 IPC 适配层。

## 架构原则

| 层级 | 职责 |
| --- | --- |
| `src/`（React） | UI、i18n、表单校验体验 — 不把密钥放进 `localStorage` |
| `src-tauri/src/commands/` | 薄 `#[tauri::command]` 包装 |
| `src-tauri/src/settings/` | 版本化非密钥配置（`settings.json`） |
| `src-tauri/src/auth/` | 版本化密钥（`auth.json`），IPC 仅返回状态 |
| `media/` | 路由 Bilibili / YouTube URL；统一预览与字幕拉取 — **已实现** |
| `bilibili/` | URL 解析、view API、字幕拉取 — **已实现** |
| `youtube/` | URL 解析、InnerTube player API、公开字幕 — **已实现** |
| `llm/` | 兼容 OpenAI 的客户端、JSON 校验 — **已实现** |
| `note/` | 内存中组装笔记、分块、Markdown — **已实现** |
| `job/` | `job_id`、取消、超时、进度事件 — **已实现** |

**不要**添加仅能编译、无实际行为的空 Rust 模块。

## 仓库布局

单应用根目录（非 monorepo）：

```
opennote/
  src/                 # React UI
  src-tauri/           # Rust 后端 + Tauri 壳
  docs/
```

## 本地配置

### 路径

| 文件 | macOS / Linux | Windows |
| --- | --- | --- |
| `settings.json` | `~/.config/opennote/settings.json`（`$XDG_CONFIG_HOME/opennote`） | `%APPDATA%\opennote\settings.json` |
| `auth.json` | `~/.local/share/opennote/auth.json`（`$XDG_DATA_HOME/opennote`） | `%LOCALAPPDATA%\opennote\auth.json` |

### `settings.json`（version 1）

非密钥字段；生产环境可记录字段名（不要记录敏感值）：

- `version`
- `base_url` — 兼容 OpenAI 的 API 根地址；仅允许 `http://` / `https://`（本地模型可用 localhost）
- `model`
- `locale` — `zh` | `en` | `system`
- `onboarding_completed`
- `notes_save_dir` — 可选绝对路径；设置后完成笔记会自动写入 Markdown 文件

**切勿**在 `settings.json` 中存储 `api_key`、Cookie 或 token。

### `auth.json`（version 1）

仅存密钥：

- `version`
- `api_key` — 兼容 OpenAI 的密钥
- `bilibili_cookie` — 可选；仅当 B 站 API 需要登录时必需

IPC 规则：前端通过 `get_auth_status` **只收到布尔值**（`has_api_key`、`has_bilibili_cookie`）。明文密钥永不返回。

### 原子写入与权限

1. 确保父目录存在。
2. 在同目录写入 `.<name>.tmp`。
3. Unix：目录 `0700`，文件 `0600`。
4. `rename` 临时文件 → 最终路径。
5. 错误信息不得回显密钥内容。

## 安全边界

### Cookie / 凭据边界

- B 站 Cookie 保存在 `auth.json`，加载后仅留在 Rust 内存。
- 只有 Rust 调用 B 站 HTTP API。
- Cookie **仅**发送到 `https://api.bilibili.com` 的 view/player 接口 — 不发往 `b23.tv` 短链跳转或字幕 CDN。
- 前端可在引导时收集一次 Cookie；保存后 UI 仅显示占位。
- 不要在日志中打印 Cookie 或 API Key。

### Bilibili URL 与 CDN 白名单

- 视频页主机（精确）：`www.bilibili.com`、`m.bilibili.com`。
- 短链主机（精确）：`b23.tv` — 手动跟随跳转并校验每跳主机，不带 Cookie，有跳转上限。
- API 主机（精确）：`api.bilibili.com`。
- 字幕 CDN 主机（后缀匹配）：`hdslb.com`、`bilibili.com`、`bilivideo.com` — 仅后缀匹配（阻止 `evil-hdslb.com`、`hdslb.com.evil`）。
- 拒绝含嵌入凭据、非 http(s) scheme，以及仅从 fragment 解析 BV/av 的 URL。

### 将 Markdown 视为不可信输入

LLM 输出与抓取的描述会以 Markdown 渲染。一律视为 **不可信**：

- 使用安全渲染器或清洗（未经审核的管道不要用原始 `dangerouslySetInnerHTML`）。
- 阻止 `javascript:`、事件处理器，以及渲染 HTML 中的远程资源加载。
- CSP 不得允许任意远程脚本（见 `tauri.conf.json`）。

### Tauri capabilities

- 主窗口仅最小 `core:default`。
- 除非功能明确需要，不要放开 `shell`、`fs`、`http` 等宽泛插件范围。
- 优先在 Rust 侧发 HTTP（`bilibili/` / `youtube/` / `llm/` 中的 `reqwest`），而不是让 webview 直连第三方。

### 代码签名 vs 更新器签名

这是 **两类不同** 问题：

| 关注点 | 目的 | 时机 |
| --- | --- | --- |
| **操作系统代码签名**（Apple Developer ID + 公证，Windows Authenticode） | 用户打开安装包/应用时减少 Gatekeeper/SmartScreen 警告 | 公开发布前 |
| **Tauri 更新器签名**（minisign 密钥对，公钥写入 `tauri.conf.json`） | 验证更新包由我们发布 | 启用自动更新前 |

内部测试可跳过 OS 代码签名。**启用自动更新前必须完成更新器签名**，即使 OS 签名延后。私钥不得提交仓库；使用 CI Secrets。

## 结构化错误

所有 command 失败返回：

```json
{ "code": "VALIDATION_ERROR", "message": "human-readable, no secrets" }
```

标准错误码：

| Code | 含义 |
| --- | --- |
| `VALIDATION_ERROR` | 输入未通过 trim/格式校验 |
| `STORAGE_ERROR` | IO / 权限 / 原子写入失败 |
| `SERIALIZATION_ERROR` | JSON 损坏或不支持的版本 |
| `NETWORK_ERROR` | HTTP 传输失败 |
| `SHORT_LINK_FAILED` | b23.tv 解析失败 |
| `VIDEO_NOT_FOUND` | View / player API：视频不存在 |
| `VIDEO_RESTRICTED` | YouTube（或类似）：需登录 / 年龄 / 地区限制；应用不支持 YouTube 账号 |
| `API_REJECTED` | 平台 API 返回拒绝 / 非零结果 |
| `PART_OUT_OF_RANGE` | 请求的 `p` 超出分 P 数量 |
| `AUTH_REQUIRED` | 可能需要登录 Cookie（例如缺少 `subtitle_url`） |
| `RATE_LIMITED` | B 站返回 HTTP 412 / 429 |
| `NO_SUBTITLE` | 无可用字幕轨道 |
| `SUBTITLE_CORRUPT` | 字幕 JSON 无效或解析后为空 |
| `LLM_AUTH_ERROR` | LLM API 拒绝凭据 |
| `LLM_RATE_LIMITED` | LLM API 限流（HTTP 429） |
| `LLM_TIMEOUT` | LLM 请求超时 |
| `LLM_NETWORK_ERROR` | LLM HTTP 传输失败 |
| `LLM_API_ERROR` | LLM API 返回错误响应 |
| `LLM_INVALID_RESPONSE` | LLM 输出未通过 JSON 或语义校验 |
| `JOB_CANCELLED` | 用户取消任务 |
| `JOB_TIMEOUT` | 任务超过截止时间 |
| `JOB_ALREADY_RUNNING` | 已有笔记任务在进行 |
| `JOB_NOT_FOUND` | 未知 `job_id` |
| `JOB_NOT_READY` | 任务仍在排队或运行中 |
| `INTERNAL_ERROR` | 未预期内部失败（锁中毒、任务 panic 等） |

前端将 code 映射为 i18n 文案；切勿直接展示原始 Rust 堆栈。

## 任务模型（`job/`）

**状态：已实现（v1 内存编排）。**

- 每个长操作分配 `job_id`（UUID）。
- 状态：`queued` → `running` → `completed` | `failed` | `cancelled`。
- **单活跃任务**：已有 `queued`/`running` 时再启动返回 `JOB_ALREADY_RUNNING`。终态任务在新任务启动时被替换。
- 命令：`start_note_job`、`cancel_job`、`get_job_status`、`get_job_result`。
- `start_note_job` 立即返回 `{ job_id }`；外层异步任务等待 `spawn_blocking`。Join 失败将任务标为 `failed`，错误码 `INTERNAL_ERROR`。
- 进度事件：`job://progress`，载荷为结构化 `JobProgress`（阶段、可选 chunk 序号/总数、终态可选 `error_code` — 不含 message/密钥）。**终态先写入 manager，再发事件**，保证 `get_job_result` 与进度事件一致。
- `get_job_result`：`completed` 返回 Markdown；`failed` 返回已存 `ErrorPayload`；`cancelled` 返回 `JOB_CANCELLED`（或已存错误）；仅 `queued`/`running` 返回 `JOB_NOT_READY`。
- `cancel_job`：设置取消标记并返回 `cancel_requested: true`；状态可能仍为 `running`，直到后台任务观察到取消。
- 任务截止：墙钟 30 分钟（`JOB_DEADLINE`）。单次 HTTP 超时不变。
- **取消语义**：内存标志在字幕拉取后、每个 LLM chunk 前后、渲染后，以及 `JobManager::complete` 内原子检查（取消优先于迟到的成功）。阻塞 HTTP 无法中途打断；若已取消，进行中请求返回后结果会被丢弃。
- 终态结果保存最终 Markdown、标题、视频 id / bvid、语言、`segment_count` — 不保存完整中间 LLM 响应。
- 全部状态仅在 Rust 内存（v1 无数据库）；退出应用即清空。

## 媒体路由（`media/`）

**状态：已实现。**

- 根据 URL 主机/形态区分 YouTube 与 Bilibili。
- `preview_video` 与 `fetch_subtitles` 共用入口；Cookie 只传入 Bilibili 路径。
- YouTube 结果适配为笔记管线已使用的字幕/预览结构。
- 工作区 UI 调用 `preview_video` / 启动任务；遗留的 `fetch_bilibili_subtitles` 仍作为仅 B 站拉取的薄命令保留。

## Bilibili 集成（`bilibili/`）

**状态：已实现（v1 字幕拉取）。**

- 在 Rust（`url.rs`）中解析 BV/av URL 与 `p` 分 P。
- 手动解析 `b23.tv` 短链，并做主机白名单校验。
- **标题来源**：B 站 **view API**（`x/web-interface/view`）。
- 字幕：`x/player/wbi/v2`（`bvid`+`cid`，与 BiliNote 一致）→ 选轨（人工中文 > AI 中文 > 任意中文 > 首个有效）→ 从白名单 CDN 下载 JSON。**不要**用带 `aid` 的 `/x/player/v2` — 可能返回空或错误视频的 `subtitle_url`。
- Cookie 仅用于 `api.bilibili.com` 请求。
- 读取前限制响应体：API/跳转约 2 MiB / 64 KiB，字幕 CDN 至多 16 MiB；超限失败且不回显响应内容。
- Tauri 命令：`fetch_bilibili_subtitles`（`commands/` 中的薄包装）。
- 单元 + mock HTTP 测试；CI 不访问真实网络。

**已知风险 / 延后**

- **Player API**：v1 使用 `/x/player/wbi/v2` + `bvid`+`cid`（对齐 BiliNote）。避免 `/x/player/v2?aid=…`，以免空列表或错误 AI 字幕 URL。空列表/空 URL 会重试一次。
- 速率限制 / ToS：见发布清单。

## YouTube 集成（`youtube/`）

**状态：已实现（v1 公开字幕）。**

- 解析 `youtube.com` / `youtu.be` / `m.youtube.com` / `music.youtube.com`（及 `*.youtube.com`）的 watch、shorts、短链。
- 通过公开 InnerTube Android player 端点（`youtubei/v1/player`）拉取元数据与字幕轨道；不使用 YouTube 账号 Cookie。
- 优先选择可用字幕轨；从白名单主机下载 timedtext / JSON3。
- 受限 / 年龄限制 / 需登录视频返回 `VIDEO_RESTRICTED` — OpenNote **不支持** YouTube 登录。
- 单元测试覆盖 URL 解析与选轨；真实网络拉取测试为可选 / 在常规 CI 中忽略。

## LLM 集成（`llm/`）

**状态：已实现（v1 chat/completions 客户端）。**

- 使用 `base_url`（API v1 根）、`api_key`、`model` 调用兼容 OpenAI 的 `chat/completions`。
- `base_url` 安全拼接到 `chat/completions`（避免 `/v1/v1` 重复）。
- 仅靠 prompt 约束 JSON（不依赖 `response_format` / `json_schema`）。
- 阻塞式 `reqwest` + rustls；连接/总超时；响应体大小上限；测试可注入 HTTP transport。
- 错误：`LLM_AUTH_ERROR`、`LLM_RATE_LIMITED`、`LLM_TIMEOUT`、`LLM_NETWORK_ERROR`、`LLM_API_ERROR`、`LLM_INVALID_RESPONSE`。
- Tauri 命令：`test_llm`（薄包装）。

## 笔记管线（`note/`）

**状态：已实现（v1 内存组装）。**

- **内部 JSON 使用整型毫秒**（`start_ms`、`end_ms`）做校验与合并；人类可读的 `HH:MM:SS`（含超过 1 小时）仅用于 prompt 与最终 Markdown。
- `chunk_segments`：保守字符预算、完整片段边界；不丢弃、不重排片段。
- 每块 LLM JSON：`{summary, sections:[{start_ms,end_ms,title,explanation}]}`，经 serde + 语义校验，并允许一次受限重试。
- 多块：规则合并/排序/去重；多块时可选再调 LLM 合并摘要。
- `render_markdown`：`# title`、摘要、分节说明、来自 **原始** 片段的全文转录（非 LLM 改写）。
- 对不可信标题与 LLM 文本做 Markdown/HTML 转义。
- 服务 API：`generate_note_data(client, metadata, segments, locale, cancellation_check, progress)` — 不耦合 Tauri。
- v1 无数据库 / 历史记录。

## yt-dlp

**延后。** 优先官方/字幕 API。若日后加入 yt-dlp：

- 以固定版本捆绑或作为 sidecar 调用。
- 严格参数白名单；绝不传入用户可控的 shell 片段。
- 单独做能力审查（子进程、临时文件）。

## 测试策略

| 区域 | 方式 |
| --- | --- |
| Paths | 纯函数 + 环境覆盖（`XDG_*`、临时目录） |
| Validation | URL、locale、model 单元测试 |
| Storage | `tempfile` 原子写入往返 |
| Auth/settings 分离 | 断言序列化 JSON 字段集合 |
| Frontend | Vitest 测 i18n 与纯 TS 辅助函数 |
| Bilibili | URL 解析、主机白名单、选轨优先级、mock HTTP 集成 |
| YouTube | URL 解析/主机检查、选轨、受限视频映射 |
| Media | 平台路由（Bilibili vs YouTube） |
| LLM / note | 端点拼接、错误映射、分块边界、校验、合并、Markdown 转义、mock e2e |
| Job | 状态机、单活跃强制、取消幂等、截止/取消映射、进度载荷安全、mock 管线编排 |
| Integration | `tauri dev` 手工；CI 单元测试不访问真实网络 |

## 发布清单（后续）

发版流水线另见 [RELEASING.zh-CN.md](RELEASING.zh-CN.md)。

- [ ] OS 代码签名（macOS + Windows）
- [ ] CI 中配置 Tauri 更新器密钥对
- [ ] CSP 复查（Markdown 渲染器已落地；若 HTML 管线变更需再审）
- [ ] 日志密钥脱敏审计
- [ ] Bilibili / YouTube ToS 与速率限制评估
