import type { UiLocale } from "../i18n";
import { progressStageLabel, t } from "../i18n";
import type { JobProgress } from "../types/job";

interface JobProgressDisplayProps {
  locale: UiLocale;
  progress: JobProgress | null;
  cancelRequested: boolean;
}

export function JobProgressDisplay({
  locale,
  progress,
  cancelRequested,
}: JobProgressDisplayProps) {
  if (!progress) {
    return null;
  }

  const stageLabel = progressStageLabel(locale, progress.stage);
  const chunkLabel =
    progress.chunk_index !== undefined && progress.chunk_count
      ? t(locale, "progressChunk", {
          current: progress.chunk_index + 1,
          total: progress.chunk_count,
        })
      : null;

  return (
    <div
      className="job-progress"
      role="status"
      aria-live="polite"
      aria-busy={progress.stage !== "done" && progress.stage !== "failed" && progress.stage !== "cancelled"}
    >
      <p className="job-progress-stage">{stageLabel}</p>
      {chunkLabel ? <p className="job-progress-chunk">{chunkLabel}</p> : null}
      {cancelRequested ? (
        <p className="job-progress-cancel">
          {t(locale, "cancelRequested")}
          <span className="muted-inline">{t(locale, "cancelRequestedHint")}</span>
        </p>
      ) : null}
    </div>
  );
}
