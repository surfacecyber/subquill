import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useNoteJob } from "./useNoteJob";

vi.mock("../api/job", () => ({
  startNoteJob: vi.fn(),
  getJobStatus: vi.fn(),
  getJobResult: vi.fn(),
  cancelJob: vi.fn(),
  listenJobProgress: vi.fn(),
}));

import {
  getJobResult,
  getJobStatus,
  listenJobProgress,
  startNoteJob,
} from "../api/job";

describe("useNoteJob listener cleanup", () => {
  afterEach(() => {
    vi.clearAllMocks();
    vi.mocked(startNoteJob).mockReset();
    vi.mocked(listenJobProgress).mockReset();
    vi.mocked(getJobStatus).mockReset();
    vi.mocked(getJobResult).mockReset();
  });

  it("unlistens when reset is called", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockResolvedValue(unlisten);
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-1",
      status: "queued",
      progress: { job_id: "job-1", stage: "fetching_subtitles" },
      batch_items: [],
    });

    const { result } = renderHook(() => useNoteJob());

    await act(async () => {
      await result.current.start(["https://example.com"]);
    });

    act(() => {
      result.current.reset();
    });

    expect(unlisten).toHaveBeenCalled();
  });

  it("fetches result after done event", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done", item_index: 0, item_total: 2 });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# hi",
      title: "hi",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
      batch_results: [
        {
          index: 0,
          url: "https://example.com/a",
          markdown: "# A",
          title: "A",
          bvid: "BV0",
          language: "zh-CN",
          segment_count: 1,
        },
        {
          index: 1,
          url: "https://example.com/b",
          markdown: "# hi",
          title: "hi",
          bvid: "BV1",
          language: "zh-CN",
          segment_count: 1,
        },
      ],
    });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-1",
      status: "completed",
      progress: { job_id: "job-1", stage: "done", item_index: 1, item_total: 2 },
      batch_items: [
        {
          index: 0,
          url: "https://example.com/a",
          status: "completed",
          title: "A",
        },
        {
          index: 1,
          url: "https://example.com/b",
          status: "completed",
          title: "hi",
        },
      ],
    });

    const { result } = renderHook(() => useNoteJob());

    await act(async () => {
      await result.current.start([
        "https://example.com/a",
        "https://example.com/b",
      ]);
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(getJobResult).toHaveBeenCalledWith("job-1");
    expect(result.current.result?.title).toBe("hi");
    expect(result.current.result?.batch_results).toHaveLength(2);
    expect(result.current.batchItems).toHaveLength(2);
    expect(startNoteJob).toHaveBeenCalledWith([
      "https://example.com/a",
      "https://example.com/b",
    ]);
  });

  it("handles done event before start response without reactivating job", async () => {
    const unlisten = vi.fn();
    let progressHandler: ((event: {
      job_id: string;
      stage: "done";
    }) => void) | null = null;

    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      progressHandler = handler;
      return unlisten;
    });
    vi.mocked(startNoteJob).mockImplementation(async () => {
      progressHandler?.({ job_id: "job-early", stage: "done" });
      return { job_id: "job-early" };
    });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# early",
      title: "early",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
    });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-early",
      status: "completed",
      progress: { job_id: "job-early", stage: "done" },
      batch_items: [],
    });

    const { result } = renderHook(() => useNoteJob());

    await act(async () => {
      await result.current.start(["https://example.com"]);
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.isActive).toBe(false);
    expect(result.current.result?.title).toBe("early");
  });

  it("ignores a second start while the first is still claiming", async () => {
    const unlisten = vi.fn();
    let resolveFirstStart!: (value: { job_id: string }) => void;
    const firstStart = new Promise<{ job_id: string }>((resolve) => {
      resolveFirstStart = resolve;
    });

    vi.mocked(listenJobProgress).mockResolvedValue(unlisten);
    vi.mocked(startNoteJob)
      .mockImplementationOnce(() => firstStart)
      .mockResolvedValueOnce({ job_id: "job-2" });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-1",
      status: "queued",
      progress: { job_id: "job-1", stage: "fetching_subtitles" },
      batch_items: [],
    });

    const { result } = renderHook(() => useNoteJob());

    let first!: Promise<void>;
    act(() => {
      first = result.current.start(["https://example.com/one"]);
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.isActive).toBe(true);

    await act(async () => {
      await result.current.start(["https://example.com/two"]);
    });

    expect(startNoteJob).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveFirstStart({ job_id: "job-1" });
      await first;
    });

    expect(result.current.jobId).toBe("job-1");
    expect(startNoteJob).toHaveBeenCalledTimes(1);
  });

  it("allows a new start after a previous job reaches a terminal state", async () => {
    const unlisten = vi.fn();
    let progressHandler:
      | ((event: {
          job_id: string;
          stage: "failed" | "calling_llm";
          error_code?: string;
        }) => void)
      | null = null;

    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      progressHandler = handler;
      return unlisten;
    });
    vi.mocked(startNoteJob)
      .mockResolvedValueOnce({ job_id: "job-1" })
      .mockResolvedValueOnce({ job_id: "job-2" });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-1",
      status: "running",
      progress: { job_id: "job-1", stage: "calling_llm" },
      batch_items: [],
    });

    const { result } = renderHook(() => useNoteJob());

    await act(async () => {
      await result.current.start(["https://example.com/one"]);
    });

    expect(result.current.jobId).toBe("job-1");
    expect(result.current.isActive).toBe(true);

    await act(async () => {
      progressHandler?.({
        job_id: "job-1",
        stage: "failed",
        error_code: "NETWORK_ERROR",
      });
      await Promise.resolve();
    });

    expect(result.current.isActive).toBe(false);
    expect(result.current.error?.code).toBe("NETWORK_ERROR");

    await act(async () => {
      await result.current.start(["https://example.com/two"]);
    });

    expect(startNoteJob).toHaveBeenCalledTimes(2);
    expect(result.current.jobId).toBe("job-2");
  });
});
