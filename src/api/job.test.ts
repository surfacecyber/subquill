import { describe, expect, it, vi } from "vitest";

import {
  cancelJob,
  getJobResult,
  getJobStatus,
  listenJobProgress,
  startNoteJob,
  testLlm,
} from "./job";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

describe("job command wrappers", () => {
  it("startNoteJob invokes start_note_job", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ job_id: "job-1" });

    await expect(startNoteJob("https://example.com")).resolves.toEqual({
      job_id: "job-1",
    });
    expect(invoke).toHaveBeenCalledWith("start_note_job", {
      url: "https://example.com",
    });
  });

  it("getJobStatus invokes get_job_status", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      job_id: "job-1",
      status: "running",
      progress: { job_id: "job-1", stage: "chunking" },
    });

    await getJobStatus("job-1");
    expect(invoke).toHaveBeenCalledWith("get_job_status", { jobId: "job-1" });
  });

  it("getJobResult invokes get_job_result", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      markdown: "# hi",
      title: "hi",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
    });

    await getJobResult("job-1");
    expect(invoke).toHaveBeenCalledWith("get_job_result", { jobId: "job-1" });
  });

  it("cancelJob invokes cancel_job", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      job_id: "job-1",
      status: "running",
      cancel_requested: true,
    });

    await cancelJob("job-1");
    expect(invoke).toHaveBeenCalledWith("cancel_job", { jobId: "job-1" });
  });

  it("testLlm invokes test_llm with input", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ ok: true, model: "deepseek-v4-flash" });

    await testLlm({
      base_url: "https://api.openai.com/v1",
      model: "deepseek-v4-flash",
      api_key: "sk-test",
    });
    expect(invoke).toHaveBeenCalledWith("test_llm", {
      input: {
        base_url: "https://api.openai.com/v1",
        model: "deepseek-v4-flash",
        api_key: "sk-test",
      },
    });
  });

  it("listenJobProgress returns unlisten", async () => {
    const unlisten = vi.fn();
    vi.mocked(listen).mockResolvedValueOnce(unlisten);

    const result = await listenJobProgress(() => {});
    expect(result).toBe(unlisten);
    expect(listen).toHaveBeenCalledWith("job://progress", expect.any(Function));
  });

  it("exportJobMarkdown invokes export_job_markdown", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ saved: true });

    const { exportJobMarkdown } = await import("./job");
    await exportJobMarkdown("job-1");
    expect(invoke).toHaveBeenCalledWith("export_job_markdown", { jobId: "job-1" });
  });
});
