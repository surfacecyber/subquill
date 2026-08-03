import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AppErrorPayload } from "../types/settings";
import type {
  CancelJobResponse,
  JobProgress,
  JobResult,
  JobStatusResponse,
  StartJobResponse,
  TestLlmSuccess,
} from "../types/job";

function toAppError(error: unknown): AppErrorPayload {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error
  ) {
    return error as AppErrorPayload;
  }

  return {
    code: "UNKNOWN_ERROR",
    message: "An unexpected error occurred",
  };
}

export async function startNoteJob(urls: string[]): Promise<StartJobResponse> {
  try {
    return await invoke<StartJobResponse>("start_note_job", { urls });
  } catch (error) {
    throw toAppError(error);
  }
}

export async function getJobStatus(jobId: string): Promise<JobStatusResponse> {
  try {
    return await invoke<JobStatusResponse>("get_job_status", { jobId });
  } catch (error) {
    throw toAppError(error);
  }
}

export async function getJobResult(jobId: string): Promise<JobResult> {
  try {
    return await invoke<JobResult>("get_job_result", { jobId });
  } catch (error) {
    throw toAppError(error);
  }
}

export async function cancelJob(jobId: string): Promise<CancelJobResponse> {
  try {
    return await invoke<CancelJobResponse>("cancel_job", { jobId });
  } catch (error) {
    throw toAppError(error);
  }
}

export interface TestLlmInput {
  base_url: string;
  model: string;
  api_key?: string;
}

export async function testLlm(input: TestLlmInput): Promise<TestLlmSuccess> {
  try {
    return await invoke<TestLlmSuccess>("test_llm", { input });
  } catch (error) {
    throw toAppError(error);
  }
}

export interface ExportJobMarkdownResponse {
  saved: boolean;
}

export async function exportJobMarkdown(
  jobId: string,
  itemIndex?: number,
): Promise<ExportJobMarkdownResponse> {
  try {
    return await invoke<ExportJobMarkdownResponse>("export_job_markdown", {
      jobId,
      itemIndex: itemIndex ?? null,
    });
  } catch (error) {
    throw toAppError(error);
  }
}

const JOB_PROGRESS_EVENT = "job://progress";

export async function listenJobProgress(
  handler: (progress: JobProgress) => void,
): Promise<UnlistenFn> {
  return listen<JobProgress>(JOB_PROGRESS_EVENT, (event) => {
    handler(event.payload);
  });
}
