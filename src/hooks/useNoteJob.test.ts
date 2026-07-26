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
  });

  it("unlistens when reset is called", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockResolvedValue(unlisten);
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });

    const { result } = renderHook(() => useNoteJob());

    await act(async () => {
      await result.current.start("https://example.com");
    });

    act(() => {
      result.current.reset();
    });

    expect(unlisten).toHaveBeenCalled();
  });

  it("fetches result after done event", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done" });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# hi",
      title: "hi",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
    });

    const { result } = renderHook(() => useNoteJob());

    await act(async () => {
      await result.current.start("https://example.com");
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(getJobResult).toHaveBeenCalledWith("job-1");
    expect(result.current.result?.title).toBe("hi");
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

    const { result } = renderHook(() => useNoteJob());

    await act(async () => {
      await result.current.start("https://example.com");
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.isActive).toBe(false);
    expect(result.current.result?.title).toBe("early");
    expect(getJobStatus).not.toHaveBeenCalled();
  });
});
