import type { AppErrorPayload } from "./settings";

export type JobStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export type JobProgressStage =
  | "fetching_subtitles"
  | "chunking"
  | "calling_llm"
  | "merging"
  | "rendering"
  | "done"
  | "failed"
  | "cancelled";

export type BatchItemStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "skipped";

export interface BatchItemView {
  index: number;
  url: string;
  status: BatchItemStatus;
  title?: string;
  /** Safe error code only — never includes message or secrets. */
  error_code?: string;
  saved_path?: string;
}

/** Successful per-URL note returned inside [`JobResult.batch_results`]. */
export interface BatchItemResult {
  index: number;
  url: string;
  markdown: string;
  title: string;
  bvid: string;
  language: string;
  segment_count: number;
  saved_path?: string | null;
}

export interface JobProgress {
  job_id: string;
  stage: JobProgressStage;
  chunk_index?: number;
  chunk_count?: number;
  /** Safe error code only — never includes message or secrets. */
  error_code?: string;
  /** 0-based index of the URL currently being processed. */
  item_index?: number;
  item_total?: number;
  item_url?: string;
}

export interface JobResult {
  markdown: string;
  title: string;
  bvid: string;
  language: string;
  segment_count: number;
  /** Absolute path when auto-save succeeded. */
  saved_path?: string | null;
  /** All successful notes for multi-URL jobs; omitted/empty for single-URL. */
  batch_results?: BatchItemResult[];
}

export interface StartJobResponse {
  job_id: string;
}

export interface JobStatusResponse {
  job_id: string;
  status: JobStatus;
  progress: JobProgress;
  error?: AppErrorPayload;
  batch_items?: BatchItemView[];
}

export interface CancelJobResponse {
  job_id: string;
  status: JobStatus;
  /** True when cancellation was requested; status may still be `running`. */
  cancel_requested: boolean;
}

export type JobErrorCode =
  | "JOB_ALREADY_RUNNING"
  | "JOB_NOT_FOUND"
  | "JOB_NOT_READY"
  | "JOB_CANCELLED"
  | "JOB_TIMEOUT"
  | "INTERNAL_ERROR";

export interface TestLlmSuccess {
  ok: boolean;
  model: string;
}
