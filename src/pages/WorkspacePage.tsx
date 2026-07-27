import { useState } from "react";

import { exportJobMarkdown } from "../api/job";
import { JobProgressDisplay } from "../components/JobProgressDisplay";
import { SafeMarkdown } from "../components/SafeMarkdown";
import { useNoteJob } from "../hooks/useNoteJob";
import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import { localizeError } from "../i18n/errors";
import { copyTextToClipboard } from "../utils/clipboard";
import type { AppErrorPayload } from "../types/settings";

interface WorkspacePageProps {
  locale: UiLocale;
}

export function WorkspacePage({ locale }: WorkspacePageProps) {
  const [url, setUrl] = useState("");
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [startError, setStartError] = useState<{
    title: string;
    detail?: string;
  } | null>(null);

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
      setStartError(localized);
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

  const displayError =
    startError ??
    (error
      ? localizeError(locale, error.code, error.message)
      : null);

  return (
    <section className="workspace" aria-labelledby="workspace-heading">
      <header className="page-header">
        <h1 id="workspace-heading">{t(locale, "navWorkspace")}</h1>
      </header>

      <div className="workspace-input">
        <label className="url-label" htmlFor="video-url">
          <span>{t(locale, "bilibiliUrl")}</span>
          <input
            id="video-url"
            type="url"
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder={t(locale, "bilibiliUrlPlaceholder")}
            disabled={isActive}
            autoComplete="off"
          />
        </label>

        <div className="workspace-actions">
          <button
            type="button"
            className="btn-primary"
            onClick={() => void handleGenerate()}
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
      </div>

      <JobProgressDisplay
        locale={locale}
        progress={progress}
        cancelRequested={cancelRequested}
      />

      {displayError ? (
        <div className="form-error-block" role="alert">
          <p className="form-error">{displayError.title}</p>
          {displayError.detail ? (
            <p className="form-error-detail">{displayError.detail}</p>
          ) : null}
        </div>
      ) : null}

      {result ? (
        <div className="preview-panel">
          <div className="preview-header">
            <h2>{t(locale, "previewTitle")}</h2>
            <div className="preview-actions">
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
            </div>
          </div>
          <div className="preview-scroll markdown-preview">
            <SafeMarkdown markdown={result.markdown} />
          </div>
        </div>
      ) : null}

      {actionMessage ? (
        <p className="form-success" role="status">{actionMessage}</p>
      ) : null}
      {actionError ? (
        <p className="form-error" role="alert">{actionError}</p>
      ) : null}
    </section>
  );
}
