import type { Locale } from "../types/settings";

export type UiLocale = "zh" | "en";

const messages = {
  zh: {
    appTitle: "OpenNote",
    navWorkspace: "生成笔记",
    navSettings: "设置",
    navAbout: "关于",
    navAriaLabel: "主导航",
    setupTitle: "首次设置",
    setupSubtitle:
      "配置 OpenAI 兼容接口与语言偏好。密钥仅保存在本机安全目录，不会显示在界面上。",
    baseUrl: "Base URL",
    model: "模型",
    apiKey: "API Key",
    apiKeySaved: "已配置",
    bilibiliCookie: "B 站 Cookie（可选，只需 SESSDATA）",
    bilibiliCookieHint:
      "用于拉取 B 站 AI/需登录字幕；YouTube 不需要。在浏览器打开 bilibili.com → F12 → Application/应用 → Cookies → bilibili.com，复制名为 SESSDATA 的值。可只贴值，也可贴 SESSDATA=值。其余 Cookie 不用复制。",
    bilibiliCookiePlaceholder: "SESSDATA=… 或直接粘贴值",
    bilibiliCookieSaved: "已配置",
    locale: "界面语言",
    localeZh: "中文",
    localeEn: "English",
    localeSystem: "跟随系统",
    save: "保存",
    saveAndContinue: "保存并继续",
    saving: "保存中…",
    saved: "设置已保存",
    requiredApiKey: "首次保存需要填写 API Key",
    testConnection: "测试连接",
    testingConnection: "测试中…",
    testConnectionSuccess: "连接成功",
    testConnectionModel: "模型",
    clearApiKey: "清除密钥",
    apiKeyCleared: "API Key 已清除",
    clearApiKeyConfirm:
      "清除已保存的 API Key？清除后需重新填写才能生成笔记。",
    clearCookie: "清除 Cookie",
    clearCookieConfirm: "清除已保存的 B 站 Cookie？部分视频之后可能无法拉取字幕。",
    cookieCleared: "Cookie 已清除",
    bilibiliCookieHowTo: "如何获取 SESSDATA？",
    notesSaveDir: "笔记保存位置",
    notesSaveDirHint:
      "可选。配置后，生成完成的笔记会自动保存为 Markdown 到该目录。留空则仅保存在应用内预览。",
    notesSaveDirPlaceholder: "未设置（不自动保存）",
    pickNotesSaveDir: "选择文件夹",
    clearNotesSaveDir: "清除位置",
    notesSaveDirCleared: "保存位置已清除",
    autoSaved: "已自动保存到 {path}",
    bilibiliUrl: "视频链接",
    bilibiliUrlPlaceholder:
      "每行一个链接，例如：\nhttps://www.bilibili.com/video/BV…\nhttps://www.youtube.com/watch?v=…",
    bilibiliUrlHint: "可一次粘贴多条链接（每行一条）。多 P 视频可在链接加 ?p=2 指定分 P。",
    workspaceEmptyHint:
      "粘贴 B 站或 YouTube 链接即可生成学习笔记。视频需有在线字幕（含 AI 字幕）；长视频可能需要几分钟。",
    workspaceEmptyCookieHint: "部分 B 站视频需配置 SESSDATA Cookie 才能拉取字幕。",
    openSettings: "打开设置",
    retryGenerate: "重试",
    generate: "生成笔记",
    generating: "生成中…",
    cancel: "取消",
    cancelRequested: "取消已请求",
    cancelRequestedHint:
      "后台任务可能仍在进行；若 HTTP 请求阻塞，停止可能稍有延迟。",
    progressEtaHint: "长视频可能需要几分钟，请耐心等待。",
    progressPercent: "{percent}%",
    videoPreviewLoading: "正在识别视频…",
    videoPreviewDuration: "时长 {duration}",
    videoPreviewPart: "第 {p} / {total} P",
    videoPreviewSubtitlesYes: "检测到字幕轨道",
    videoPreviewSubtitlesNo: "未检测到字幕轨道",
    videoPreviewAuthRequired:
      "可能需要配置 B 站 Cookie（SESSDATA）才能拉取字幕",
    videoPreviewAuthStale: "B 站 Cookie 已失效，请到设置中更新 SESSDATA",
    saveDirGuide:
      "生成成功。可在设置中配置笔记保存位置，之后将自动保存为 Markdown。未配置时请及时另存或复制，关闭或开始新任务后结果会丢失。",
    saveDirGuideBatch:
      "批量笔记已保留在本次任务中，可点选上方条目切换查看。未配置保存位置时请及时另存或复制；也可去设置开启自动保存。",
    saveDirGuideDismiss: "知道了",
    batchItemSelect: "查看此笔记",
    batchSelected: "当前预览",
    batchUnsavedHint: "未自动保存",
    batchSavedHint: "已保存",
    notificationDismiss: "关闭",
    unsavedSettingsConfirm: "设置尚未保存，确定离开并丢弃更改？",
    copyMarkdown: "复制 Markdown",
    exportMarkdown: "另存为 .md",
    revealInFolder: "在文件夹中显示",
    revealFailed: "无法打开文件夹，请手动前往保存路径。",
    copied: "已复制到剪贴板",
    copyFailed: "无法访问剪贴板。请手动选择预览内容复制。",
    exportSaved: "已保存 Markdown 文件",
    exportCancelled: "已取消保存",
    exportFailed: "导出失败",
    previewTitle: "笔记预览",
    segmentCount: "{count} 段字幕",
    loading: "加载中…",
    loadSettingsFailed: "无法加载设置。",
    aboutTitle: "关于 OpenNote",
    aboutDescription: "从 B 站 / YouTube 视频字幕生成学习笔记的桌面应用。",
    aboutPlatformsTitle: "支持平台",
    aboutPlatforms: "Bilibili、YouTube（公开字幕）。",
    aboutPrivacyTitle: "隐私",
    aboutPrivacy:
      "API Key 与 B 站 Cookie 仅保存在本机，不会上传到 OpenNote 服务器。Cookie 仅用于请求 B 站官方 API。",
    aboutCookieTitle: "Cookie 用途",
    aboutCookie:
      "可选的 SESSDATA 用于拉取需登录或 AI 字幕；YouTube 无需 Cookie。",
    version: "版本",
    previewExpand: "展开预览",
    previewCollapse: "收起预览",
    updaterStatus: "自动更新",
    updaterStatusIdle: "尚未检查",
    updaterStatusChecking: "正在检查更新…",
    updaterStatusUpToDate: "已是最新版本",
    updaterStatusAvailable: "发现新版本 {version}",
    updaterStatusDownloading: "正在下载… {progress}%",
    updaterStatusDownloadingIndeterminate: "正在下载更新…",
    updaterStatusInstalling: "正在安装更新…",
    updaterStatusUnconfigured: "当前构建未配置自动更新",
    updaterCheckNow: "检查更新",
    updaterInstall: "安装并重启",
    updaterDismiss: "稍后",
    updaterBannerAvailable: "发现新版本 {version}，是否现在安装？",
    updaterErrorCheck: "检查更新失败，请稍后重试。",
    updaterErrorDownload: "下载更新失败，请稍后重试。",
    updaterErrorInstall: "安装更新失败，请稍后重试。",
    updaterErrorNetwork: "无法连接更新服务器，请检查网络后重试。",
    progressFetching: "拉取字幕",
    progressChunking: "分块字幕",
    progressLlm: "调用模型",
    progressMerging: "合并结果",
    progressRendering: "渲染 Markdown",
    progressDone: "完成",
    progressFailed: "失败",
    progressCancelled: "已取消",
    progressChunk: "分块 {current} / {total}",
    progressBatchItem: "视频 {current} / {total}",
    batchItemPending: "等待中",
    batchItemRunning: "生成中",
    batchItemCompleted: "已完成",
    batchItemFailed: "失败",
    batchItemCancelled: "已取消",
    batchItemSkipped: "已跳过",
    batchSummary: "批量结果",
    settingsTitle: "设置",
    settingsSubtitle:
      "更新接口配置、凭据与笔记保存位置。留空 API Key 表示保留已存密钥。",
    errorGeneric: "操作失败，请检查后重试。",
    errorAuthSaveFailed: "凭据保存失败，设置尚未更新。",
    errorSettingsSaveFailed: "设置保存失败（凭据可能已更新）。",
    error_storage: "本地存储或读写失败。请检查磁盘权限后重试。",
    error_validation: "输入校验失败。请检查格式后重试。",
    error_NETWORK_ERROR: "网络请求失败。请检查连接后重试。",
    error_bilibili: "视频字幕或元信息获取失败。",
    error_NO_SUBTITLE:
      "该视频暂无可用的在线字幕（含 AI 字幕）。通常与 Cookie 无关；可换有字幕的视频，或稍后再试。",
    error_SUBTITLE_CORRUPT:
      "拉取到的字幕与视频时长明显不匹配（可能是 B 站 AI 字幕串台或未生成完整）。请稍后重试，或换有人工字幕的视频。",
    error_VIDEO_NOT_FOUND: "未找到该视频。",
    error_VIDEO_RESTRICTED:
      "该 YouTube 视频需要登录，或受年龄/地区限制。多数公开视频无需登录；OpenNote 不支持登录 YouTube。",
    error_AUTH_REQUIRED: "需要配置 B 站 SESSDATA Cookie 才能获取字幕。",
    error_RATE_LIMITED: "B 站请求过于频繁，请稍后再试。",
    error_PART_OUT_OF_RANGE: "分 P 编号超出视频页数。",
    error_SHORT_LINK_FAILED: "短链接解析失败，请改用完整视频链接。",
    error_API_REJECTED: "视频平台拒绝了请求，请稍后重试或检查链接。",
    error_llm: "模型接口调用失败。请检查 Base URL、模型与 API Key。",
    error_LLM_AUTH_ERROR: "模型 API 认证失败，请检查 API Key。",
    error_LLM_RATE_LIMITED: "模型 API 触发限流，请稍后再试。",
    error_LLM_TIMEOUT: "模型接口请求超时，请稍后重试或换更快的模型。",
    error_LLM_INVALID_RESPONSE: "模型返回内容无法解析，请更换模型或重试。",
    error_job: "笔记生成任务失败或已取消。",
    error_JOB_TIMEOUT: "笔记生成超时，请稍后重试。",
    error_JOB_CANCELLED: "笔记生成已取消。",
    error_JOB_ALREADY_RUNNING: "已有笔记任务在进行中，请等待完成或先取消。",
    error_unknown: "发生未知错误。",
    errorAuthLoadFailed: "无法读取已保存的凭据，请重新填写 API Key 后保存。",
    authPartialSuccess: "部分保存成功：请查看上方提示。",
  },
  en: {
    appTitle: "OpenNote",
    navWorkspace: "Generate notes",
    navSettings: "Settings",
    navAbout: "About",
    navAriaLabel: "Main",
    setupTitle: "Initial setup",
    setupSubtitle:
      "Configure your OpenAI-compatible endpoint and language. Secrets stay on disk and are never shown in the UI.",
    baseUrl: "Base URL",
    model: "Model",
    apiKey: "API Key",
    apiKeySaved: "Configured",
    bilibiliCookie: "Bilibili cookie (optional, SESSDATA only)",
    bilibiliCookieHint:
      "Used to fetch Bilibili AI / login-gated captions; YouTube does not need it. On bilibili.com open DevTools → Application → Cookies → bilibili.com, copy the SESSDATA value. Paste either the raw value or SESSDATA=your-value. Other cookies are not needed.",
    bilibiliCookiePlaceholder: "SESSDATA=… or paste value",
    bilibiliCookieSaved: "Configured",
    locale: "Language",
    localeZh: "中文",
    localeEn: "English",
    localeSystem: "Follow system",
    save: "Save",
    saveAndContinue: "Save and continue",
    saving: "Saving…",
    saved: "Settings saved",
    requiredApiKey: "API key is required on first save",
    testConnection: "Test connection",
    testingConnection: "Testing…",
    testConnectionSuccess: "Connection successful",
    testConnectionModel: "Model",
    clearApiKey: "Clear API key",
    apiKeyCleared: "API key cleared",
    clearApiKeyConfirm:
      "Clear the saved API key? You'll need to enter it again before generating notes.",
    clearCookie: "Clear cookie",
    clearCookieConfirm:
      "Clear the saved Bilibili cookie? Some videos may no longer provide captions.",
    cookieCleared: "Cookie cleared",
    bilibiliCookieHowTo: "How do I get SESSDATA?",
    notesSaveDir: "Notes save location",
    notesSaveDirHint:
      "Optional. When set, generated notes are saved as Markdown files in this folder. Leave empty to keep notes in-app only.",
    notesSaveDirPlaceholder: "Not set (no auto-save)",
    pickNotesSaveDir: "Choose folder",
    clearNotesSaveDir: "Clear location",
    notesSaveDirCleared: "Save location cleared",
    autoSaved: "Auto-saved to {path}",
    bilibiliUrl: "Video URL",
    bilibiliUrlPlaceholder:
      "One URL per line, e.g.\nhttps://www.bilibili.com/video/BV…\nhttps://www.youtube.com/watch?v=…",
    bilibiliUrlHint:
      "Paste multiple links (one per line). For multi-part Bilibili videos, add ?p=2 to pick a part.",
    workspaceEmptyHint:
      "Paste a Bilibili or YouTube link to generate study notes. The video needs online captions (including AI captions). Long videos may take a few minutes.",
    workspaceEmptyCookieHint:
      "Some Bilibili videos need a SESSDATA cookie before captions can be fetched.",
    openSettings: "Open settings",
    retryGenerate: "Retry",
    generate: "Generate notes",
    generating: "Generating…",
    cancel: "Cancel",
    cancelRequested: "Cancel requested",
    cancelRequestedHint:
      "The background task may still be running; blocked HTTP calls can stop with a short delay.",
    progressEtaHint: "Long videos may take a few minutes — hang tight.",
    progressPercent: "{percent}%",
    videoPreviewLoading: "Looking up video…",
    videoPreviewDuration: "Duration {duration}",
    videoPreviewPart: "Part {p} / {total}",
    videoPreviewSubtitlesYes: "Caption track found",
    videoPreviewSubtitlesNo: "No caption track found",
    videoPreviewAuthRequired:
      "A Bilibili SESSDATA cookie may be required for captions",
    videoPreviewAuthStale:
      "Bilibili cookie expired — update SESSDATA in Settings",
    saveDirGuide:
      "Notes are ready. Set a save folder in Settings to auto-save Markdown next time. Without a folder, export or copy soon — starting a new job clears in-memory notes.",
    saveDirGuideBatch:
      "Batch notes stay in this job — click a completed item above to switch the preview. Without a save folder, export or copy soon, or configure auto-save in Settings.",
    saveDirGuideDismiss: "Got it",
    batchItemSelect: "View this note",
    batchSelected: "Showing",
    batchUnsavedHint: "Not auto-saved",
    batchSavedHint: "Saved",
    notificationDismiss: "Dismiss",
    unsavedSettingsConfirm: "Settings are unsaved. Leave and discard changes?",
    copyMarkdown: "Copy Markdown",
    exportMarkdown: "Save as .md",
    revealInFolder: "Show in folder",
    revealFailed: "Could not open the folder. Open the save path manually.",
    copied: "Copied to clipboard",
    copyFailed: "Clipboard is unavailable. Select preview text to copy manually.",
    exportSaved: "Markdown file saved",
    exportCancelled: "Save cancelled",
    exportFailed: "Export failed",
    previewTitle: "Note preview",
    segmentCount: "{count} subtitle segments",
    loading: "Loading…",
    loadSettingsFailed: "Failed to load settings.",
    aboutTitle: "About OpenNote",
    aboutDescription:
      "Desktop app that generates study notes from Bilibili / YouTube video subtitles.",
    aboutPlatformsTitle: "Supported platforms",
    aboutPlatforms: "Bilibili and YouTube (public captions).",
    aboutPrivacyTitle: "Privacy",
    aboutPrivacy:
      "API keys and Bilibili cookies stay on this device. OpenNote does not upload them to its own servers. The cookie is sent only to Bilibili official APIs.",
    aboutCookieTitle: "Cookie usage",
    aboutCookie:
      "Optional SESSDATA unlocks login-gated or AI captions on Bilibili. YouTube does not need a cookie.",
    version: "Version",
    previewExpand: "Expand preview",
    previewCollapse: "Collapse preview",
    updaterStatus: "Auto-update",
    updaterStatusIdle: "Not checked yet",
    updaterStatusChecking: "Checking for updates…",
    updaterStatusUpToDate: "You're up to date",
    updaterStatusAvailable: "Update available: {version}",
    updaterStatusDownloading: "Downloading… {progress}%",
    updaterStatusDownloadingIndeterminate: "Downloading update…",
    updaterStatusInstalling: "Installing update…",
    updaterStatusUnconfigured: "This build is not configured for auto-update",
    updaterCheckNow: "Check for updates",
    updaterInstall: "Install and restart",
    updaterDismiss: "Later",
    updaterBannerAvailable: "Version {version} is available. Install now?",
    updaterErrorCheck: "Could not check for updates. Try again later.",
    updaterErrorDownload: "Could not download the update. Try again later.",
    updaterErrorInstall: "Could not install the update. Try again later.",
    updaterErrorNetwork: "Could not reach the update server. Check your network.",
    progressFetching: "Fetching subtitles",
    progressChunking: "Chunking subtitles",
    progressLlm: "Calling model",
    progressMerging: "Merging results",
    progressRendering: "Rendering Markdown",
    progressDone: "Done",
    progressFailed: "Failed",
    progressCancelled: "Cancelled",
    progressChunk: "Chunk {current} / {total}",
    progressBatchItem: "Video {current} / {total}",
    batchItemPending: "Pending",
    batchItemRunning: "Generating",
    batchItemCompleted: "Done",
    batchItemFailed: "Failed",
    batchItemCancelled: "Cancelled",
    batchItemSkipped: "Skipped",
    batchSummary: "Batch results",
    settingsTitle: "Settings",
    settingsSubtitle:
      "Update endpoint settings, credentials, and notes save location. Leave API key empty to keep the saved key.",
    errorGeneric: "Something went wrong. Check your input and try again.",
    errorAuthSaveFailed: "Credentials could not be saved. Settings were not updated.",
    errorSettingsSaveFailed:
      "Settings could not be saved (credentials may have been updated).",
    error_storage: "Local storage read/write failed. Check disk permissions.",
    error_validation: "Validation failed. Check your input format.",
    error_NETWORK_ERROR: "Network request failed. Check your connection and try again.",
    error_bilibili: "Failed to fetch video subtitles or metadata.",
    error_NO_SUBTITLE:
      "No online subtitle track is available for this video (including AI captions). This usually isn't a cookie issue; try another video or retry later.",
    error_SUBTITLE_CORRUPT:
      "Fetched subtitles do not match the video duration (Bilibili AI captions may be wrong or incomplete). Retry later, or use a video with human-made captions.",
    error_VIDEO_NOT_FOUND: "Video not found.",
    error_VIDEO_RESTRICTED:
      "This YouTube video requires sign-in or is age/region restricted. Most public videos need no login; OpenNote does not support YouTube accounts.",
    error_AUTH_REQUIRED: "A Bilibili SESSDATA cookie is required to fetch subtitles.",
    error_RATE_LIMITED: "Bilibili rate limit reached. Try again later.",
    error_PART_OUT_OF_RANGE: "The requested part number exceeds the video page count.",
    error_SHORT_LINK_FAILED: "Could not resolve the short link. Use the full video URL instead.",
    error_API_REJECTED: "The video platform rejected the request. Retry later or check the URL.",
    error_llm: "Model API call failed. Check base URL, model, and API key.",
    error_LLM_AUTH_ERROR: "Model API authentication failed. Check your API key.",
    error_LLM_RATE_LIMITED: "Model API rate limit reached. Try again later.",
    error_LLM_TIMEOUT: "Model API request timed out. Retry later or use a faster model.",
    error_LLM_INVALID_RESPONSE: "Model response could not be parsed. Try another model.",
    error_job: "Note generation failed or was cancelled.",
    error_JOB_TIMEOUT: "Note generation timed out. Try again later.",
    error_JOB_CANCELLED: "Note generation was cancelled.",
    error_JOB_ALREADY_RUNNING: "A note job is already running. Wait for it or cancel first.",
    error_unknown: "An unknown error occurred.",
    errorAuthLoadFailed:
      "Could not read saved credentials. Enter your API key again and save.",
    authPartialSuccess: "Partial save: see messages above.",
  },
} as const;

export type MessageKey = keyof (typeof messages)["en"];

export function resolveUiLocale(
  locale: Locale,
  systemLocale: string = navigator.language,
): UiLocale {
  if (locale === "zh" || locale === "en") {
    return locale;
  }

  return systemLocale.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export function t(
  locale: UiLocale,
  key: MessageKey,
  params?: Record<string, string | number>,
): string {
  let text: string = messages[locale][key];
  if (params) {
    for (const [name, value] of Object.entries(params)) {
      text = text.replace(`{${name}}`, String(value));
    }
  }
  return text;
}

export function progressStageLabel(
  locale: UiLocale,
  stage: string,
): string {
  const map: Record<string, MessageKey> = {
    fetching_subtitles: "progressFetching",
    chunking: "progressChunking",
    calling_llm: "progressLlm",
    merging: "progressMerging",
    rendering: "progressRendering",
    done: "progressDone",
    failed: "progressFailed",
    cancelled: "progressCancelled",
  };
  const key = map[stage];
  return key ? t(locale, key) : stage;
}

/** Approximate overall progress for wait-state UX (0–100). */
export function progressPercent(stage: string, chunkIndex?: number, chunkCount?: number): number {
  const stageStart: Record<string, number> = {
    fetching_subtitles: 0,
    chunking: 15,
    calling_llm: 25,
    merging: 85,
    rendering: 92,
    done: 100,
    failed: 100,
    cancelled: 100,
  };
  const stageSpan: Record<string, number> = {
    fetching_subtitles: 15,
    chunking: 10,
    calling_llm: 60,
    merging: 7,
    rendering: 8,
  };

  const start = stageStart[stage];
  if (start === undefined) {
    return 0;
  }

  if (stage === "calling_llm" && chunkCount && chunkCount > 0) {
    const index = Math.min(Math.max(chunkIndex ?? 0, 0), chunkCount - 1);
    return Math.min(99, Math.round(start + (stageSpan.calling_llm * index) / chunkCount));
  }

  return start;
}
