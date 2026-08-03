import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";

import { getAuthStatus, previewVideo, revealInFolder } from "../api/commands";
import { exportJobMarkdown } from "../api/job";
import { JobProgressDisplay } from "../components/JobProgressDisplay";
import { SafeMarkdown } from "../components/SafeMarkdown";
import { useNoteJob } from "../hooks/useNoteJob";
import type { UiLocale } from "../i18n";
import { t, type MessageKey } from "../i18n";
import { localizeError } from "../i18n/errors";
import { useNotify } from "../notifications/NotificationProvider";
import { copyTextToClipboard } from "../utils/clipboard";
import type { VideoPreview } from "../types/bilibili";
import type { BatchItemResult, BatchItemStatus } from "../types/job";
import type { AppErrorPayload } from "../types/settings";

interface WorkspacePageProps {
  locale: UiLocale;
  notesSaveDir?: string | null;
  onOpenSettings?: () => void;
  onJobActiveChange?: (active: boolean) => void;
  /** When true (e.g. workspace tab visible), refresh cookie status for auth tips. */
  active?: boolean;
}

type DisplayedNote = {
  markdown: string;
  title: string;
  bvid: string;
  language: string;
  segment_count: number;
  saved_path?: string | null;
};

function toDisplayedNote(
  note:
    | Pick<
        BatchItemResult,
        "markdown" | "title" | "bvid" | "language" | "segment_count" | "saved_path"
      >
    | null
    | undefined,
): DisplayedNote | null {
  if (!note) {
    return null;
  }
  return {
    markdown: note.markdown,
    title: note.title,
    bvid: note.bvid,
    language: note.language,
    segment_count: note.segment_count,
    saved_path: note.saved_path,
  };
}

function formatDuration(ms: number): string {
  const totalSec = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  const seconds = totalSec % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function looksLikeVideoUrl(value: string): boolean {
  return /^https?:\/\//i.test(value.trim());
}

/** Unique video URLs from a multi-line paste (one per line). */
function parseVideoUrls(value: string): string[] {
  const urls: string[] = [];
  const seen = new Set<string>();
  for (const line of value.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!looksLikeVideoUrl(trimmed) || seen.has(trimmed)) {
      continue;
    }
    seen.add(trimmed);
    urls.push(trimmed);
  }
  return urls;
}

type UrlPreviewState = {
  url: string;
  preview: VideoPreview | null;
  error: string | null;
};

/** Path prefix check for reveal CTA; normalizes separators for Windows. */
function isPathInsideDir(filePath: string, dirPath: string): boolean {
  const file = filePath.replace(/\\/g, "/");
  const dir = dirPath.replace(/\\/g, "/").replace(/\/+$/, "");
  return file === dir || file.startsWith(`${dir}/`);
}

function batchItemStatusLabel(locale: UiLocale, status: BatchItemStatus): string {
  const map: Record<BatchItemStatus, MessageKey> = {
    pending: "batchItemPending",
    running: "batchItemRunning",
    completed: "batchItemCompleted",
    failed: "batchItemFailed",
    cancelled: "batchItemCancelled",
    skipped: "batchItemSkipped",
  };
  return t(locale, map[status]);
}

export function WorkspacePage({
  locale,
  notesSaveDir = null,
  onOpenSettings,
  onJobActiveChange,
  active = true,
}: WorkspacePageProps) {
  const { notify } = useNotify();
  const [url, setUrl] = useState("");
  const [startError, setStartError] = useState<{
    code?: string;
    title: string;
    detail?: string;
  } | null>(null);
  const [urlPreviews, setUrlPreviews] = useState<UrlPreviewState[]>([]);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [dismissSaveGuide, setDismissSaveGuide] = useState(false);
  const [previewExpanded, setPreviewExpanded] = useState(false);
  const [hasBilibiliCookie, setHasBilibiliCookie] = useState(false);
  const [previewRefreshKey, setPreviewRefreshKey] = useState(0);
  const [selectedBatchIndex, setSelectedBatchIndex] = useState<number | null>(
    null,
  );
  const previewSeqRef = useRef(0);
  const wasActiveRef = useRef(active);
  const skipPreviewDebounceRef = useRef(false);
  const lastAutoSavedPathRef = useRef<string | null>(null);

  function refreshPreviews() {
    skipPreviewDebounceRef.current = true;
    setPreviewRefreshKey((key) => key + 1);
  }

  const {
    jobId,
    progress,
    result,
    error,
    batchItems,
    isActive,
    cancelRequested,
    start,
    cancel,
  } = useNoteJob();

  const hasValidVideoUrl = parseVideoUrls(url).length > 0;
  const batchResults = result?.batch_results ?? [];
  const selectedBatchResult =
    selectedBatchIndex === null
      ? null
      : (batchResults.find((item) => item.index === selectedBatchIndex) ?? null);
  const displayedNote: DisplayedNote | null =
    toDisplayedNote(selectedBatchResult) ?? toDisplayedNote(result);

  useEffect(() => {
    onJobActiveChange?.(isActive);
    return () => {
      onJobActiveChange?.(false);
    };
  }, [isActive, onJobActiveChange]);

  useEffect(() => {
    if (!result) {
      setSelectedBatchIndex(null);
      return;
    }
    const results = result.batch_results ?? [];
    if (results.length === 0) {
      setSelectedBatchIndex(null);
      return;
    }
    setSelectedBatchIndex((prev) => {
      if (prev !== null && results.some((item) => item.index === prev)) {
        return prev;
      }
      return results[results.length - 1]?.index ?? null;
    });
  }, [result]);

  useEffect(() => {
    if (!active) {
      return;
    }
    let cancelled = false;
    void getAuthStatus()
      .then((status) => {
        if (!cancelled) {
          setHasBilibiliCookie(status.has_bilibili_cookie);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setHasBilibiliCookie(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [active]);

  // Returning from settings (or any inactive tab): force-refetch previews so
  // cookie updates are reflected, not only the auth tip copy.
  useEffect(() => {
    if (active && !wasActiveRef.current) {
      refreshPreviews();
    }
    wasActiveRef.current = active;
  }, [active]);

  useEffect(() => {
    const savedPath = displayedNote?.saved_path;
    if (!savedPath || savedPath === lastAutoSavedPathRef.current) {
      return;
    }
    lastAutoSavedPathRef.current = savedPath;
    notify("success", t(locale, "autoSaved", { path: savedPath }));
  }, [displayedNote?.saved_path, locale, notify]);

  useEffect(() => {
    setPreviewExpanded(false);
  }, [displayedNote?.markdown]);

  useEffect(() => {
    const urls = parseVideoUrls(url);
    if (urls.length === 0 || isActive) {
      previewSeqRef.current += 1;
      setUrlPreviews([]);
      setPreviewLoading(false);
      return;
    }

    const seq = ++previewSeqRef.current;
    setPreviewLoading(true);
    const delay = skipPreviewDebounceRef.current ? 0 : 600;
    skipPreviewDebounceRef.current = false;

    const timer = window.setTimeout(() => {
      void Promise.all(
        urls.map(async (previewUrl): Promise<UrlPreviewState> => {
          try {
            const preview = await previewVideo(previewUrl);
            return { url: previewUrl, preview, error: null };
          } catch (err) {
            const payload = err as AppErrorPayload;
            const localized = localizeError(locale, payload.code, payload.message);
            return { url: previewUrl, preview: null, error: localized.title };
          }
        }),
      ).then((previews) => {
        if (previewSeqRef.current !== seq) {
          return;
        }
        setUrlPreviews(previews);
        setPreviewLoading(false);
      });
    }, delay);

    return () => {
      window.clearTimeout(timer);
    };
  }, [url, isActive, locale, previewRefreshKey]);

  async function handleGenerate() {
    setStartError(null);
    lastAutoSavedPathRef.current = null;
    setSelectedBatchIndex(null);
    setDismissSaveGuide(false);

    const urls = parseVideoUrls(url);
    if (urls.length === 0) {
      notify("error", t(locale, "invalidVideoUrls"));
      return;
    }

    try {
      await start(urls);
    } catch (err) {
      const payload = err as AppErrorPayload;
      const localized = localizeError(locale, payload.code, payload.message);
      setStartError({ code: payload.code, ...localized });
    }
  }

  async function handleCopy() {
    if (!displayedNote?.markdown) {
      return;
    }

    try {
      await copyTextToClipboard(displayedNote.markdown);
      notify("success", t(locale, "copied"));
    } catch {
      notify("error", t(locale, "copyFailed"));
    }
  }

  async function handleExport() {
    if (!jobId) {
      return;
    }

    try {
      const response = await exportJobMarkdown(
        jobId,
        selectedBatchIndex ?? undefined,
      );
      if (response.saved) {
        notify("success", t(locale, "exportSaved"));
      } else {
        notify("info", t(locale, "exportCancelled"));
      }
    } catch (err) {
      const payload = err as AppErrorPayload;
      const localized = localizeError(locale, payload.code, payload.message);
      notify("error", localized.title);
    }
  }

  async function handleReveal() {
    if (!displayedNote?.saved_path) {
      return;
    }

    try {
      await revealInFolder(displayedNote.saved_path);
    } catch {
      notify("error", t(locale, "revealFailed"));
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (isActive || url.trim().length === 0) {
      return;
    }
    void handleGenerate();
  }

  const displayError =
    startError ??
    (error
      ? { code: error.code, ...localizeError(locale, error.code, error.message) }
      : null);
  const needsSettingsCta = displayError?.code === "AUTH_REQUIRED";
  const showEmptyHint = !displayedNote && !isActive && !displayError;
  const showProgress = Boolean(progress) && (isActive || !displayedNote);
  const hasBatchResults = batchResults.length > 0;
  const showSaveDirGuide =
    Boolean(displayedNote) &&
    !notesSaveDir &&
    !displayedNote?.saved_path &&
    !dismissSaveGuide;
  const canRevealSavedPath = Boolean(
    displayedNote?.saved_path &&
      notesSaveDir &&
      isPathInsideDir(displayedNote.saved_path, notesSaveDir),
  );

  return (
    <section className="workspace" aria-labelledby="workspace-heading">
      <header className="page-header">
        <h1 id="workspace-heading">{t(locale, "navWorkspace")}</h1>
      </header>

      <form className="workspace-input workspace-docked" onSubmit={handleSubmit}>
        <div className="workspace-body">
          {showEmptyHint ? (
            <aside className="workspace-empty" aria-label={t(locale, "navWorkspace")}>
              <p className="muted">{t(locale, "workspaceEmptyHint")}</p>
              <p className="muted workspace-empty-cookie">
                {t(locale, "workspaceEmptyCookieHint")}
              </p>
              {onOpenSettings ? (
                <button
                  type="button"
                  className="btn-secondary"
                  onClick={onOpenSettings}
                >
                  {t(locale, "openSettings")}
                </button>
              ) : null}
            </aside>
          ) : null}

          <label className="url-label" htmlFor="video-url">
            <span>{t(locale, "bilibiliUrl")}</span>
            <textarea
              id="video-url"
              name="video-url"
              rows={4}
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              placeholder={t(locale, "bilibiliUrlPlaceholder")}
              disabled={isActive}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <p className="muted-inline url-hint">{t(locale, "bilibiliUrlHint")}</p>

          {previewLoading ? (
            <p className="muted video-preview-status" role="status">
              {t(locale, "videoPreviewLoading")}
            </p>
          ) : null}

          {!previewLoading && urlPreviews.length > 0 ? (
            <div className="video-preview-list" aria-live="polite">
              {urlPreviews.map((item) =>
                item.preview ? (
                  <aside key={item.url} className="video-preview">
                    <p className="video-preview-title">{item.preview.title}</p>
                    <p className="muted video-preview-meta">
                      {t(locale, "videoPreviewDuration", {
                        duration: formatDuration(item.preview.duration_ms),
                      })}
                      {item.preview.page_count > 1
                        ? ` · ${t(locale, "videoPreviewPart", {
                            p: item.preview.p,
                            total: item.preview.page_count,
                          })}`
                        : null}
                      {item.preview.part_title
                        ? ` · ${item.preview.part_title}`
                        : null}
                    </p>
                    {item.preview.has_subtitles ? (
                      <p className="video-preview-subs ok">
                        {t(locale, "videoPreviewSubtitlesYes")}
                      </p>
                    ) : item.preview.auth_required ? null : (
                      <p className="video-preview-subs warn">
                        {t(locale, "videoPreviewSubtitlesNo")}
                      </p>
                    )}
                    {item.preview.auth_required ? (
                      <>
                        <p className="video-preview-subs warn">
                          {t(
                            locale,
                            hasBilibiliCookie
                              ? "videoPreviewAuthStale"
                              : "videoPreviewAuthRequired",
                          )}
                        </p>
                        <div className="error-actions">
                          <button
                            type="button"
                            className="btn-secondary"
                            onClick={refreshPreviews}
                          >
                            {t(locale, "retryGenerate")}
                          </button>
                        </div>
                      </>
                    ) : null}
                  </aside>
                ) : (
                  <aside key={item.url} className="video-preview">
                    <p className="muted video-preview-status">
                      {item.error ?? t(locale, "videoPreviewSubtitlesNo")}
                    </p>
                    <div className="error-actions">
                      <button
                        type="button"
                        className="btn-secondary"
                        onClick={refreshPreviews}
                      >
                        {t(locale, "retryGenerate")}
                      </button>
                    </div>
                  </aside>
                ),
              )}
            </div>
          ) : null}

          {showProgress ? (
            <JobProgressDisplay
              locale={locale}
              progress={progress}
              cancelRequested={cancelRequested}
            />
          ) : null}

          {batchItems.length > 1 ? (
            <aside className="batch-summary" aria-label={t(locale, "batchSummary")}>
              <p className="batch-summary-title">{t(locale, "batchSummary")}</p>
              <ul className="batch-summary-list">
                {batchItems.map((item) => {
                  const selectable = batchResults.some(
                    (note) => note.index === item.index,
                  );
                  const isSelected =
                    selectable && selectedBatchIndex === item.index;
                  const content = (
                    <>
                      <span
                        className={`batch-summary-status status-${item.status}`}
                      >
                        {batchItemStatusLabel(locale, item.status)}
                      </span>
                      <span className="batch-summary-meta">
                        {item.title ?? item.url}
                        {item.error_code ? (
                          <span className="muted-inline">
                            {" "}
                            · {localizeError(locale, item.error_code, "").title}
                          </span>
                        ) : null}
                        {selectable ? (
                          <span className="muted-inline">
                            {" "}
                            ·{" "}
                            {item.saved_path ||
                            batchResults.find((note) => note.index === item.index)
                              ?.saved_path
                              ? t(locale, "batchSavedHint")
                              : t(locale, "batchUnsavedHint")}
                            {isSelected
                              ? ` · ${t(locale, "batchSelected")}`
                              : ""}
                          </span>
                        ) : null}
                      </span>
                    </>
                  );

                  return (
                    <li
                      key={`${item.index}-${item.url}`}
                      className={`batch-summary-item${isSelected ? " is-selected" : ""}`}
                    >
                      {selectable ? (
                        <button
                          type="button"
                          className="batch-summary-select"
                          aria-pressed={isSelected}
                          aria-label={t(locale, "batchItemSelect")}
                          onClick={() => setSelectedBatchIndex(item.index)}
                        >
                          {content}
                        </button>
                      ) : (
                        content
                      )}
                    </li>
                  );
                })}
              </ul>
            </aside>
          ) : null}

          {displayError ? (
            <div className="form-error-block" role="alert">
              <p className="form-error">{displayError.title}</p>
              {displayError.detail ? (
                <p className="form-error-detail">{displayError.detail}</p>
              ) : null}
              <div className="error-actions">
                <button
                  type="button"
                  className="btn-secondary"
                  onClick={() => void handleGenerate()}
                  disabled={isActive || !hasValidVideoUrl}
                >
                  {t(locale, "retryGenerate")}
                </button>
                {needsSettingsCta && onOpenSettings ? (
                  <button
                    type="button"
                    className="btn-secondary"
                    onClick={onOpenSettings}
                  >
                    {t(locale, "openSettings")}
                  </button>
                ) : null}
              </div>
            </div>
          ) : null}

          {showSaveDirGuide ? (
            <aside className="save-dir-guide" role="status">
              <p className="muted">
                {t(
                  locale,
                  hasBatchResults ? "saveDirGuideBatch" : "saveDirGuide",
                )}
              </p>
              <div className="form-row">
                {onOpenSettings ? (
                  <button
                    type="button"
                    className="btn-secondary"
                    onClick={onOpenSettings}
                  >
                    {t(locale, "openSettings")}
                  </button>
                ) : null}
                <button
                  type="button"
                  className="btn-secondary"
                  onClick={() => setDismissSaveGuide(true)}
                >
                  {t(locale, "saveDirGuideDismiss")}
                </button>
              </div>
            </aside>
          ) : null}

          {displayedNote ? (
            <div className="preview-panel">
              <div className="preview-header">
                <div className="preview-heading">
                  <h2>{displayedNote.title || t(locale, "previewTitle")}</h2>
                  <p className="muted preview-meta">
                    {t(locale, "segmentCount", {
                      count: displayedNote.segment_count,
                    })}
                  </p>
                </div>
                <div className="preview-actions">
                  <button
                    type="button"
                    className="btn-secondary"
                    onClick={() => setPreviewExpanded((value) => !value)}
                  >
                    {previewExpanded
                      ? t(locale, "previewCollapse")
                      : t(locale, "previewExpand")}
                  </button>
                  <button
                    type="button"
                    className="btn-secondary"
                    onClick={() => void handleCopy()}
                  >
                    {t(locale, "copyMarkdown")}
                  </button>
                  <button
                    type="button"
                    className="btn-secondary"
                    onClick={() => void handleExport()}
                  >
                    {t(locale, "exportMarkdown")}
                  </button>
                  {canRevealSavedPath ? (
                    <button
                      type="button"
                      className="btn-secondary"
                      onClick={() => void handleReveal()}
                    >
                      {t(locale, "revealInFolder")}
                    </button>
                  ) : null}
                </div>
              </div>
              <div
                className={`preview-scroll markdown-preview${previewExpanded ? " preview-scroll-expanded" : ""}`}
              >
                <SafeMarkdown markdown={displayedNote.markdown} />
              </div>
            </div>
          ) : null}
        </div>

        <div className="workspace-actions">
          <button
            type="submit"
            className="btn-primary"
            disabled={isActive || !hasValidVideoUrl}
          >
            {isActive
              ? t(locale, "generating")
              : t(locale, "generate")}
          </button>
          {isActive ? (
            <button
              type="button"
              className="btn-secondary"
              onClick={() => void cancel()}
              disabled={cancelRequested}
            >
              {cancelRequested
                ? t(locale, "cancelRequested")
                : t(locale, "cancel")}
            </button>
          ) : null}
        </div>
      </form>
    </section>
  );
}
