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

vi.mock("../api/commands", () => ({
  revealInFolder: vi.fn(),
  previewVideo: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { previewVideo, revealInFolder } from "../api/commands";
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
    vi.mocked(previewVideo).mockRejectedValue({
      code: "VALIDATION_ERROR",
      message: "skip preview in tests",
    });
  });

  it("shows empty-state guidance before the first result", () => {
    render(<WorkspacePage locale="en" onOpenSettings={vi.fn()} />);

    expect(
      screen.getByText(/Paste a Bilibili or YouTube link/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Open settings" }),
    ).toBeInTheDocument();
  });

  it("opens settings from empty-state CTA", async () => {
    const onOpenSettings = vi.fn();
    const user = userEvent.setup();
    render(<WorkspacePage locale="en" onOpenSettings={onOpenSettings} />);

    await user.click(screen.getByRole("button", { name: "Open settings" }));
    expect(onOpenSettings).toHaveBeenCalled();
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

  it("shows 1-based chunk progress and percent", async () => {
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
      expect(screen.getByText("25%")).toBeInTheDocument();
      expect(screen.getByRole("progressbar")).toBeInTheDocument();
    });
  });

  it("fetches result when job completes and shows metadata", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done" });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# Hello",
      title: "Hello Video",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 12,
      saved_path: "/tmp/opennote-notes/Hello Video.md",
    });

    const user = userEvent.setup();
    render(
      <WorkspacePage locale="en" notesSaveDir="/tmp/opennote-notes" />,
    );

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(getJobResult).toHaveBeenCalledWith("job-1");
      expect(
        screen.getByRole("heading", { name: "Hello Video" }),
      ).toBeInTheDocument();
      expect(screen.getByText("12 subtitle segments")).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "Show in folder" }),
      ).toBeInTheDocument();
    });
  });

  it("hides reveal when saved path is outside the current notes folder", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done" });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# Hello",
      title: "Hello Video",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
      saved_path: "/tmp/old-notes/Hello Video.md",
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" notesSaveDir="/tmp/new-notes" />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(
        screen.getByRole("heading", { name: "Hello Video" }),
      ).toBeInTheDocument();
    });

    expect(
      screen.queryByRole("button", { name: "Show in folder" }),
    ).not.toBeInTheDocument();
  });

  it("reveals auto-saved file in folder", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done" });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# Hello",
      title: "Hello Video",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
      saved_path: "/tmp/notes/Hello Video.md",
    });
    vi.mocked(revealInFolder).mockResolvedValue();

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" notesSaveDir="/tmp/notes" />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Show in folder" }),
      ).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Show in folder" }));
    expect(revealInFolder).toHaveBeenCalledWith("/tmp/notes/Hello Video.md");
  });

  it("offers settings CTA when AUTH_REQUIRED", async () => {
    const onOpenSettings = vi.fn();
    vi.mocked(startNoteJob).mockRejectedValue({
      code: "AUTH_REQUIRED",
      message: "cookie needed",
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" onOpenSettings={onOpenSettings} />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(
        screen.getByText(/SESSDATA cookie is required/i),
      ).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Open settings" }));
    expect(onOpenSettings).toHaveBeenCalled();
  });

  it("retries generation from the error CTA", async () => {
    vi.mocked(startNoteJob)
      .mockRejectedValueOnce({
        code: "NETWORK_ERROR",
        message: "offline",
      })
      .mockResolvedValueOnce({ job_id: "job-2" });
    vi.mocked(getJobStatus).mockResolvedValue({
      job_id: "job-2",
      status: "running",
      progress: { job_id: "job-2", stage: "fetching_subtitles" },
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Retry" }));

    await waitFor(() => {
      expect(startNoteJob).toHaveBeenCalledTimes(2);
    });
  });

  it("shows lightweight video preview metadata", async () => {
    vi.mocked(previewVideo).mockResolvedValue({
      title: "Demo Video",
      platform: "bilibili",
      video_id: "BV1xx",
      duration_ms: 125000,
      p: 2,
      page_count: 3,
      part_title: "Part Two",
      has_subtitles: true,
      auth_required: false,
    });

    const user = userEvent.setup();
    render(<WorkspacePage locale="en" />);

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx?p=2",
    );

    await waitFor(
      () => {
        expect(screen.getByText("Demo Video")).toBeInTheDocument();
        expect(screen.getByText(/Duration 2:05/)).toBeInTheDocument();
        expect(screen.getByText(/Part 2 \/ 3/)).toBeInTheDocument();
        expect(screen.getByText("Caption track found")).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
  });

  it("shows save-folder guide after success when auto-save is unset", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done" });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# Hello",
      title: "Hello Video",
      bvid: "BV1",
      language: "zh-CN",
      segment_count: 1,
      saved_path: null,
    });

    const onOpenSettings = vi.fn();
    const user = userEvent.setup();
    render(
      <WorkspacePage
        locale="en"
        notesSaveDir={null}
        onOpenSettings={onOpenSettings}
      />,
    );

    await user.type(
      screen.getByLabelText("Video URL"),
      "https://www.bilibili.com/video/BV1xx",
    );
    await user.click(screen.getByRole("button", { name: "Generate notes" }));

    await waitFor(() => {
      expect(
        screen.getByText(/Set a save folder in Settings/i),
      ).toBeInTheDocument();
    });
  });

  it("expands and collapses the note preview", async () => {
    const unlisten = vi.fn();
    vi.mocked(listenJobProgress).mockImplementation(async (handler) => {
      handler({ job_id: "job-1", stage: "done" });
      return unlisten;
    });
    vi.mocked(startNoteJob).mockResolvedValue({ job_id: "job-1" });
    vi.mocked(getJobResult).mockResolvedValue({
      markdown: "# Hello\n\nLong note body",
      title: "Hello Video",
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
      expect(
        screen.getByRole("button", { name: "Expand preview" }),
      ).toBeInTheDocument();
    });

    const scroll = document.querySelector(".preview-scroll");
    expect(scroll).not.toHaveClass("preview-scroll-expanded");

    await user.click(screen.getByRole("button", { name: "Expand preview" }));
    expect(scroll).toHaveClass("preview-scroll-expanded");
    expect(
      screen.getByRole("button", { name: "Collapse preview" }),
    ).toBeInTheDocument();
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
