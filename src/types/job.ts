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

export interface JobProgress {
  job_id: string;
  stage: JobProgressStage;
  chunk_index?: number;
  chunk_count?: number;
  /** Safe error code only — never includes message or secrets. */
  error_code?: string;
}

export interface JobResult {
  markdown: string;
  title: string;
  bvid: string;
  language: string;
  segment_count: number;
  /** Absolute path when auto-save succeeded. */
  saved_path?: string | null;
}

export interface StartJobResponse {
  job_id: string;
}

export interface JobStatusResponse {
  job_id: string;
  status: JobStatus;
  progress: JobProgress;
  error?: AppErrorPayload;
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
