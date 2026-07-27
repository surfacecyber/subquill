import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { WorkspacePage } from "./WorkspacePage";

vi.mock("../api/job", () => ({
  startNoteJob: vi.fn(),
  getJobStatus: vi.fn(),
  getJobResult: vi.fn(),
  cancelJob: vi.fn(),
  listenJobProgress: vi.fn(),
  exportJobMarkdown: vi.fn(),
  testLlm: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import {
  cancelJob,
  getJobResult,
  getJobStatus,
  listenJobProgress,
  startNoteJob,
} from "../api/job";

describe("WorkspacePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listenJobProgress).mockResolvedValue(vi.fn());
  });

  it("disables generate while job is active", async () => {
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-1",
      status: "running",
      progress: { job_id: "job-1", stage: "chunking" },
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" />);

    const input = screen.getByLabelText("Video URL");
    await user.type(input, "https://www.bilibili.com/video/BV1xx");

    const generateButton = screen.getByRole("button", { name: "Generate notes" });
    await user.click(generateButton);

    await waitFor(() => {
      expect(generateButton).toBeDisabled();
    });
  });

  it("shows 1-based chunk progress", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({
        job_id: "job-1",
        stage: "calling_llm",
        chunk_index: 0,
        chunk_count: 3,
      });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-1",
      status: "running",
      progress: {
        job_id: "job-1",
        stage: "calling_llm",
        chunk_index: 0,
        chunk_count: 3,
      },
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(screen.getByText("Chunk 1 / 3")).toBeInTheDocument();
    });
  });

  it("fetches result when job completes", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done" });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# Hello",
      title: "Hello",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(getJobResult).toHaveBeenCalledWith("job-1");
      expect(screen.getByRole("heading", { name: "Hello" })).toBeInTheDocument();
    });
  });

  it("shows cancel requested state", async () => {
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(cancelJob).mockResolvedValue({
      job_id: "job-1",
      status: "running",
      cancel_requested: true,
    });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-1",
      status: "running",
      progress: { job_id: "job-1", stage: "calling_llm" },
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(screen.getAllByText("Cancel requested").length).toBeGreaterThan(0);
    });
  });
});
