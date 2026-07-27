import type { Locale } from "../types/settings";

export type UiLocale = "zh" | "en";

const messages = {
  zh: {
    appTitle: "OpenNote",
    navWorkspace: "笔记",
    navSettings: "设置",
    navAbout: "关于",
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
    clearCookie: "清除 Cookie",
    cookieCleared: "Cookie 已清除",
    bilibiliUrl: "视频链接",
    bilibiliUrlPlaceholder:
      "https://www.bilibili.com/video/BV… 或 https://www.youtube.com/watch?v=…",
    generate: "生成笔记",
    generating: "生成中…",
    cancel: "取消",
    cancelRequested: "取消已请求",
    cancelRequestedHint:
      "后台任务可能仍在进行；若 HTTP 请求阻塞，停止可能稍有延迟。",
    copyMarkdown: "复制 Markdown",
    exportMarkdown: "另存为 .md",
    copied: "已复制到剪贴板",
    copyFailed: "无法访问剪贴板。请手动选择预览内容复制。",
    exportSaved: "已保存 Markdown 文件",
    exportCancelled: "已取消保存",
    exportFailed: "导出失败",
    previewTitle: "笔记预览",
    loading: "加载中…",
    loadSettingsFailed: "无法加载设置。",
    aboutTitle: "关于 OpenNote",
    aboutDescription: "从 B 站 / YouTube 视频字幕生成学习笔记的桌面应用。",
    version: "版本",
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
    settingsTitle: "设置",
    settingsSubtitle: "更新接口配置与凭据。留空 API Key 表示保留已存密钥。",
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
    error_AUTH_REQUIRED: "需要配置 B 站 SESSDATA Cookie 才能获取字幕。",
    error_RATE_LIMITED: "B 站请求过于频繁，请稍后再试。",
    error_PART_OUT_OF_RANGE: "分 P 编号超出视频页数。",
    error_llm: "模型接口调用失败。请检查 Base URL、模型与 API Key。",
    error_LLM_AUTH_ERROR: "模型 API 认证失败，请检查 API Key。",
    error_LLM_RATE_LIMITED: "模型 API 触发限流，请稍后再试。",
    error_LLM_INVALID_RESPONSE: "模型返回内容无法解析，请更换模型或重试。",
    error_job: "笔记生成任务失败或已取消。",
    error_JOB_TIMEOUT: "笔记生成超时，请稍后重试。",
    error_JOB_CANCELLED: "笔记生成已取消。",
    error_unknown: "发生未知错误。",
    authPartialSuccess: "部分保存成功：请查看上方提示。",
  },
  en: {
    appTitle: "OpenNote",
    navWorkspace: "Notes",
    navSettings: "Settings",
    navAbout: "About",
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
    clearCookie: "Clear cookie",
    cookieCleared: "Cookie cleared",
    bilibiliUrl: "Video URL",
    bilibiliUrlPlaceholder:
      "https://www.bilibili.com/video/BV… or https://www.youtube.com/watch?v=…",
    generate: "Generate notes",
    generating: "Generating…",
    cancel: "Cancel",
    cancelRequested: "Cancel requested",
    cancelRequestedHint:
      "The background task may still be running; blocked HTTP calls can stop with a short delay.",
    copyMarkdown: "Copy Markdown",
    exportMarkdown: "Save as .md",
    copied: "Copied to clipboard",
    copyFailed: "Clipboard is unavailable. Select preview text to copy manually.",
    exportSaved: "Markdown file saved",
    exportCancelled: "Save cancelled",
    exportFailed: "Export failed",
    previewTitle: "Note preview",
    loading: "Loading…",
    loadSettingsFailed: "Failed to load settings.",
    aboutTitle: "About OpenNote",
    aboutDescription:
      "Desktop app that generates study notes from Bilibili / YouTube video subtitles.",
    version: "Version",
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
    settingsTitle: "Settings",
    settingsSubtitle:
      "Update endpoint settings and credentials. Leave API key empty to keep the saved key.",
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
    error_AUTH_REQUIRED: "A Bilibili SESSDATA cookie is required to fetch subtitles.",
    error_RATE_LIMITED: "Bilibili rate limit reached. Try again later.",
    error_PART_OUT_OF_RANGE: "The requested part number exceeds the video page count.",
    error_llm: "Model API call failed. Check base URL, model, and API key.",
    error_LLM_AUTH_ERROR: "Model API authentication failed. Check your API key.",
    error_LLM_RATE_LIMITED: "Model API rate limit reached. Try again later.",
    error_LLM_INVALID_RESPONSE: "Model response could not be parsed. Try another model.",
    error_job: "Note generation failed or was cancelled.",
    error_JOB_TIMEOUT: "Note generation timed out. Try again later.",
    error_JOB_CANCELLED: "Note generation was cancelled.",
    error_unknown: "An unknown error occurred.",
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
