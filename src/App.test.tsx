import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";

vi.mock("./api/commands", () => ({
  getSettings: vi.fn(),
  getAuthStatus: vi.fn(),
  saveAuth: vi.fn(),
  saveSettings: vi.fn(),
  getAppInfo: vi.fn(),
  revealInFolder: vi.fn(),
  previewVideo: vi.fn(),
}));

vi.mock("./api/job", () => ({
  startNoteJob: vi.fn(),
  getJobStatus: vi.fn(),
  getJobResult: vi.fn(),
  cancelJob: vi.fn(),
  listenJobProgress: vi.fn(),
  testLlm: vi.fn(),
  exportJobMarkdown: vi.fn(),
}));

import { getSettings, getAuthStatus, getAppInfo } from "./api/commands";
import { listenJobProgress } from "./api/job";

const completedSettings = {
  version: 1,
  base_url: "https://api.openai.com/v1",
  model: "deepseek-v4-flash",
  locale: "en" as const,
  onboarding_completed: true,
  notes_save_dir: null,
};

function mainNav() {
  return screen.getByRole("navigation", { name: "Main" });
}

describe("App view persistence", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getSettings).mockResolvedValue(completedSettings);
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(getAppInfo).mockResolvedValue({
      name: "OpenNote",
      version: "0.1.0",
    });
    vi.mocked(listenJobProgress).mockResolvedValue(vi.fn());
  });

  it("keeps workspace mounted but hidden when switching views", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByLabelText("Video URL")).toBeVisible();
    });

    await user.click(within(mainNav()).getByRole("button", { name: "Settings" }));

    const urlInput = screen.getByLabelText("Video URL");
    expect(urlInput).toBeInTheDocument();
    expect(urlInput).not.toBeVisible();

    const workspacePanel = urlInput.closest(".view-panel");
    expect(workspacePanel).toHaveAttribute("hidden");
    expect(workspacePanel).toHaveAttribute("aria-hidden", "true");
  });

  it("confirms before leaving settings with unsaved edits", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByLabelText("Video URL")).toBeVisible();
    });

    await user.click(within(mainNav()).getByRole("button", { name: "Settings" }));
    await waitFor(() => {
      expect(screen.getByLabelText("Base URL")).toBeVisible();
    });

    await user.type(screen.getByLabelText("Model"), "-edited");
    await user.click(
      within(mainNav()).getByRole("button", { name: "Generate notes" }),
    );

    expect(confirmSpy).toHaveBeenCalled();
    expect(screen.getByLabelText("Base URL")).toBeVisible();

    confirmSpy.mockRestore();
  });

  it("does not prompt when settings edits are reverted", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByLabelText("Video URL")).toBeVisible();
    });

    await user.click(within(mainNav()).getByRole("button", { name: "Settings" }));
    const model = await screen.findByLabelText("Model");
    await user.clear(model);
    await user.type(model, "temp-model");
    await user.clear(model);
    await user.type(model, completedSettings.model);

    await user.click(
      within(mainNav()).getByRole("button", { name: "Generate notes" }),
    );

    expect(confirmSpy).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Video URL")).toBeVisible();

    confirmSpy.mockRestore();
  });

  it("marks the active nav item with aria-current", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByLabelText("Video URL")).toBeVisible();
    });

    expect(
      within(mainNav()).getByRole("button", { name: "Generate notes" }),
    ).toHaveAttribute("aria-current", "page");

    await user.click(within(mainNav()).getByRole("button", { name: "About" }));
    expect(
      within(mainNav()).getByRole("button", { name: "About" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      within(mainNav()).getByRole("button", { name: "Generate notes" }),
    ).not.toHaveAttribute("aria-current");
  });
});
