import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";

vi.mock("./api/commands", () => ({
  getSettings: vi.fn(),
  getAuthStatus: vi.fn(),
  saveAuth: vi.fn(),
  saveSettings: vi.fn(),
  getAppInfo: vi.fn(),
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

import { getSettings, getAuthStatus } from "./api/commands";
import { listenJobProgress } from "./api/job";

const completedSettings = {
  version: 1,
  base_url: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  locale: "en" as const,
  onboarding_completed: true,
};

describe("App view persistence", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getSettings).mockResolvedValue(completedSettings);
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(listenJobProgress).mockResolvedValue(vi.fn());
  });

  it("keeps workspace mounted but hidden when switching views", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByLabelText("Bilibili video URL")).toBeVisible();
    });

    await user.click(screen.getByRole("button", { name: "Settings" }));

    const urlInput = screen.getByLabelText("Bilibili video URL");
    expect(urlInput).toBeInTheDocument();
    expect(urlInput).not.toBeVisible();

    const workspacePanel = urlInput.closest(".view-panel");
    expect(workspacePanel).toHaveAttribute("hidden");
    expect(workspacePanel).toHaveAttribute("aria-hidden", "true");
  });
});
