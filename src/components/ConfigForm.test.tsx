import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ConfigForm } from "./ConfigForm";

vi.mock("../api/commands", () => ({
  getAuthStatus: vi.fn(),
  saveAuth: vi.fn(),
  saveSettings: vi.fn(),
}));

vi.mock("../api/job", () => ({
  testLlm: vi.fn(),
}));

import { getAuthStatus, saveAuth, saveSettings } from "../api/commands";
import { testLlm } from "../api/job";

const baseSettings = {
  version: 1,
  base_url: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  locale: "en" as const,
  onboarding_completed: false,
};

describe("ConfigForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: false,
      has_bilibili_cookie: false,
    });
  });

  it("requires api key when none is saved", async () => {
    const user = userEvent.setup();
    render(
      <ConfigForm
        mode="onboarding"
        locale="en"
        initialSettings={baseSettings}
        onSettingsSaved={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Save and continue" }));

    expect(screen.getByText("API key is required on first save")).toBeInTheDocument();
    expect(saveAuth).not.toHaveBeenCalled();
  });

  it("shows test connection success", async () => {
    vi.mocked(testLlm).mockResolvedValue({ ok: true, model: "gpt-4o-mini" });

    const user = userEvent.setup();
    render(
      <ConfigForm
        mode="settings"
        locale="en"
        initialSettings={{ ...baseSettings, onboarding_completed: true }}
        onSettingsSaved={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Test connection" }));

    await waitFor(() => {
      expect(screen.getByText(/Connection successful/)).toBeInTheDocument();
    });

    expect(testLlm).toHaveBeenCalledWith({
      base_url: "https://api.openai.com/v1",
      model: "gpt-4o-mini",
      api_key: undefined,
    });
  });

  it("tests connection with unsaved onboarding form values", async () => {
    vi.mocked(testLlm).mockResolvedValue({ ok: true, model: "gpt-4o-mini" });

    const user = userEvent.setup();
    render(
      <ConfigForm
        mode="onboarding"
        locale="en"
        initialSettings={baseSettings}
        onSettingsSaved={vi.fn()}
      />,
    );

    await user.clear(screen.getByLabelText("Base URL"));
    await user.type(
      screen.getByLabelText("Base URL"),
      "http://localhost:11434/v1",
    );
    await user.clear(screen.getByLabelText("Model"));
    await user.type(screen.getByLabelText("Model"), "llama3");
    await user.type(screen.getByLabelText("API Key"), "sk-test-key");

    await user.click(screen.getByRole("button", { name: "Test connection" }));

    await waitFor(() => {
      expect(testLlm).toHaveBeenCalledWith({
        base_url: "http://localhost:11434/v1",
        model: "llama3",
        api_key: "sk-test-key",
      });
    });
    expect(saveAuth).not.toHaveBeenCalled();
  });

  it("does not pretend full success when settings save fails after auth", async () => {
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(saveAuth).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(saveSettings).mockRejectedValue({
      code: "VALIDATION_ERROR",
      message: "bad url",
    });

    const user = userEvent.setup();
    render(
      <ConfigForm
        mode="settings"
        locale="en"
        initialSettings={{ ...baseSettings, onboarding_completed: true }}
        onSettingsSaved={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(saveAuth).toHaveBeenCalled();
      expect(screen.getByText(/Settings could not be saved/)).toBeInTheDocument();
    });
  });
});
