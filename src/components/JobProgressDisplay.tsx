import type { UiLocale } from "../i18n";
import { progressPercent, progressStageLabel, t } from "../i18n";
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
  const batchTotal = progress.item_total ?? 0;
  const batchLabel =
    batchTotal > 1 && progress.item_index !== undefined
      ? t(locale, "progressBatchItem", {
          current: progress.item_index + 1,
          total: batchTotal,
        })
      : null;
  const percent = progressPercent(
    progress.stage,
    progress.chunk_index,
    progress.chunk_count,
  );
  const isBusy =
    progress.stage !== "done" &&
    progress.stage !== "failed" &&
    progress.stage !== "cancelled";

  return (
    <div
      className="job-progress"
      role="status"
      aria-live="polite"
      aria-busy={isBusy}
    >
      <div className="job-progress-header">
        <p className="job-progress-stage">{stageLabel}</p>
        {isBusy ? (
          <p className="job-progress-percent">
            {t(locale, "progressPercent", { percent })}
          </p>
        ) : null}
      </div>
      {batchLabel ? (
        <p className="job-progress-batch">
          {batchLabel}
          {progress.item_url ? (
            <span className="muted-inline job-progress-batch-url">
              {" "}
              · {progress.item_url}
            </span>
          ) : null}
        </p>
      ) : null}
      {isBusy ? (
        <div
          className="job-progress-bar"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent}
          aria-label={stageLabel}
        >
          <div
            className="job-progress-bar-fill"
            style={{ width: `${percent}%` }}
          />
        </div>
      ) : null}
      {chunkLabel ? <p className="job-progress-chunk">{chunkLabel}</p> : null}
      {isBusy ? (
        <p className="job-progress-eta muted-inline">
          {t(locale, "progressEtaHint")}
        </p>
      ) : null}
      {cancelRequested ? (
        <p className="job-progress-cancel">
          {t(locale, "cancelRequested")}
          <span className="muted-inline">{t(locale, "cancelRequestedHint")}</span>
        </p>
      ) : null}
    </div>
  );
}
