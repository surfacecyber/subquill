import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";

import { previewVideo, revealInFolder } from "../api/commands";
import { exportJobMarkdown } from "../api/job";
import { JobProgressDisplay } from "../components/JobProgressDisplay";
import { SafeMarkdown } from "../components/SafeMarkdown";
import { useNoteJob } from "../hooks/useNoteJob";
import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import { localizeError } from "../i18n/errors";
import { copyTextToClipboard } from "../utils/clipboard";
import type { VideoPreview } from "../types/bilibili";
import type { AppErrorPayload } from "../types/settings";

interface WorkspacePageProps {
  locale: UiLocale;
  notesSaveDir?: string | null;
  onOpenSettings?: () => void;
  onJobActiveChange?: (active: boolean) => void;
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

export function WorkspacePage({
  locale,
  notesSaveDir = null,
  onOpenSettings,
  onJobActiveChange,
}: WorkspacePageProps) {
  const [url, setUrl] = useState("");
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [startError, setStartError] = useState<{
    code?: string;
    title: string;
    detail?: string;
  } | null>(null);
  const [videoPreview, setVideoPreview] = useState<VideoPreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [dismissSaveGuide, setDismissSaveGuide] = useState(false);
  const [previewExpanded, setPreviewExpanded] = useState(false);
  const previewSeqRef = useRef(0);

  const {
    jobId,
    progress,
    result,
    error,
    isActive,
    cancelRequested,
    start,
    cancel,
  } = useNoteJob();

  useEffect(() => {
    onJobActiveChange?.(isActive);
    return () => {
      onJobActiveChange?.(false);
    };
  }, [isActive, onJobActiveChange]);

  useEffect(() => {
    if (result?.saved_path) {
      setActionMessage(t(locale, "autoSaved", { path: result.saved_path }));
    }
  }, [result?.saved_path, locale]);

  useEffect(() => {
    setPreviewExpanded(false);
  }, [result?.markdown]);

  useEffect(() => {
    const trimmed = url.trim();
    if (!looksLikeVideoUrl(trimmed) || isActive) {
      previewSeqRef.current += 1;
      setVideoPreview(null);
      setPreviewLoading(false);
      setPreviewError(null);
      return;
    }

    const seq = ++previewSeqRef.current;
    setPreviewLoading(true);
    setPreviewError(null);

    const timer = window.setTimeout(() => {
      void previewVideo(trimmed)
        .then((preview) => {
          if (previewSeqRef.current !== seq) {
            return;
          }
          setVideoPreview(preview);
          setPreviewLoading(false);
        })
        .catch((err) => {
          if (previewSeqRef.current !== seq) {
            return;
          }
          setVideoPreview(null);
          setPreviewLoading(false);
          const payload = err as AppErrorPayload;
          const localized = localizeError(locale, payload.code, payload.message);
          setPreviewError(localized.title);
        });
    }, 600);

    return () => {
      window.clearTimeout(timer);
    };
  }, [url, isActive, locale]);

  async function handleGenerate() {
    setStartError(null);
    setActionMessage(null);
    setActionError(null);

    const trimmed = url.trim();
    if (!trimmed) {
      return;
    }

    try {
      await start(trimmed);
    } catch (err) {
      const payload = err as AppErrorPayload;
      const localized = localizeError(locale, payload.code, payload.message);
      setStartError({ code: payload.code, ...localized });
    }
  }

  async function handleCopy() {
    if (!result?.markdown) {
      return;
    }

    setActionMessage(null);
    setActionError(null);

    try {
      await copyTextToClipboard(result.markdown);
      setActionMessage(t(locale, "copied"));
    } catch {
      setActionError(t(locale, "copyFailed"));
    }
  }

  async function handleExport() {
    if (!jobId) {
      return;
    }

    setActionMessage(null);
    setActionError(null);

    try {
      const response = await exportJobMarkdown(jobId);
      if (response.saved) {
        setActionMessage(t(locale, "exportSaved"));
      } else {
        setActionMessage(t(locale, "exportCancelled"));
      }
    } catch (err) {
      const payload = err as AppErrorPayload;
      const localized = localizeError(locale, payload.code, payload.message);
      setActionError(localized.title);
    }
  }

  async function handleReveal() {
    if (!result?.saved_path) {
      return;
    }

    setActionError(null);

    try {
      await revealInFolder(result.saved_path);
    } catch {
      setActionError(t(locale, "revealFailed"));
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
  const showEmptyHint = !result && !isActive && !displayError;
  const showProgress = Boolean(progress) && (isActive || !result);
  const showSaveDirGuide =
    Boolean(result) &&
    !notesSaveDir &&
    !result?.saved_path &&
    !dismissSaveGuide;

  return (
    <section className="workspace" aria-labelledby="workspace-heading">
      <header className="page-header">
        <h1 id="workspace-heading">{t(locale, "navWorkspace")}</h1>
      </header>

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

      <form className="workspace-input" onSubmit={handleSubmit}>
        <label className="url-label" htmlFor="video-url">
          <span>{t(locale, "bilibiliUrl")}</span>
          <input
            id="video-url"
            type="url"
            name="video-url"
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

        {videoPreview && !previewLoading ? (
          <aside className="video-preview" aria-live="polite">
            <p className="video-preview-title">{videoPreview.title}</p>
            <p className="muted video-preview-meta">
              {t(locale, "videoPreviewDuration", {
                duration: formatDuration(videoPreview.duration_ms),
              })}
              {videoPreview.page_count > 1
                ? ` · ${t(locale, "videoPreviewPart", {
                    p: videoPreview.p,
                    total: videoPreview.page_count,
                  })}`
                : null}
              {videoPreview.part_title
                ? ` · ${videoPreview.part_title}`
                : null}
            </p>
            <p
              className={
                videoPreview.has_subtitles
                  ? "video-preview-subs ok"
                  : "video-preview-subs warn"
              }
            >
              {videoPreview.has_subtitles
                ? t(locale, "videoPreviewSubtitlesYes")
                : t(locale, "videoPreviewSubtitlesNo")}
            </p>
            {videoPreview.auth_required ? (
              <p className="video-preview-subs warn">
                {t(locale, "videoPreviewAuthRequired")}
              </p>
            ) : null}
          </aside>
        ) : null}

        {previewError && !previewLoading && !videoPreview ? (
          <p className="muted video-preview-status">{previewError}</p>
        ) : null}

        <div className="workspace-actions">
          <button
            type="submit"
            className="btn-primary"
            disabled={isActive || url.trim().length === 0}
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

      {showProgress ? (
        <JobProgressDisplay
          locale={locale}
          progress={progress}
          cancelRequested={cancelRequested}
        />
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
              disabled={isActive || url.trim().length === 0}
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
          <p className="muted">{t(locale, "saveDirGuide")}</p>
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

      {result ? (
        <div className="preview-panel">
          <div className="preview-header">
            <div className="preview-heading">
              <h2>{result.title || t(locale, "previewTitle")}</h2>
              <p className="muted preview-meta">
                {t(locale, "segmentCount", { count: result.segment_count })}
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
              {result.saved_path ? (
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
          {actionMessage ? (
            <p className="preview-status form-success" role="status">
              {actionMessage}
            </p>
          ) : null}
          {actionError ? (
            <p className="preview-status form-error" role="alert">
              {actionError}
            </p>
          ) : null}
          <div
            className={`preview-scroll markdown-preview${previewExpanded ? " preview-scroll-expanded" : ""}`}
          >
            <SafeMarkdown markdown={result.markdown} />
          </div>
        </div>
      ) : (
        <>
          {actionMessage ? (
            <p className="form-success" role="status">
              {actionMessage}
            </p>
          ) : null}
          {actionError ? (
            <p className="form-error" role="alert">
              {actionError}
            </p>
          ) : null}
        </>
      )}
    </section>
  );
}
