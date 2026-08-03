import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ConfigForm } from "./ConfigForm";

vi.mock("../api/commands", () => ({
  getAuthStatus: vi.fn(),
  saveAuth: vi.fn(),
  saveSettings: vi.fn(),
  pickNotesSaveDir: vi.fn(),
}));

vi.mock("../api/job", () => ({
  testLlm: vi.fn(),
}));

import { getAuthStatus, pickNotesSaveDir, saveAuth, saveSettings } from "../api/commands";
import { testLlm } from "../api/job";

const baseSettings = {
  version: 1,
  base_url: "https://api.openai.com/v1",
  model: "deepseek-v4-flash",
  locale: "en" as const,
  onboarding_completed: false,
  notes_save_dir: null,
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
    vi.mocked(testLlm).mockResolvedValue({ ok: true, model: "deepseek-v4-flash" });

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
      model: "deepseek-v4-flash",
      api_key: undefined,
    });
  });

  it("tests connection with unsaved onboarding form values", async () => {
    vi.mocked(testLlm).mockResolvedValue({ ok: true, model: "deepseek-v4-flash" });

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

  it("shows SESSDATA cookie guidance", () => {
    render(
      <ConfigForm
        mode="settings"
        locale="en"
        initialSettings={{ ...baseSettings, onboarding_completed: true }}
        onSettingsSaved={vi.fn()}
      />,
    );

    expect(
      screen.getByText(/Bilibili cookie \(optional, SESSDATA only\)/),
    ).toBeInTheDocument();
    expect(screen.getByText(/Used to fetch Bilibili AI/i)).toBeInTheDocument();
    expect(screen.getByText(/copy the SESSDATA value/i)).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("SESSDATA=… or paste value"),
    ).toBeInTheDocument();
  });

  it("clears saved feedback when the form is edited again", async () => {
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(saveAuth).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(saveSettings).mockResolvedValue({
      ...baseSettings,
      onboarding_completed: true,
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
      expect(screen.getByText("Settings saved")).toBeInTheDocument();
    });

    await user.clear(screen.getByLabelText("Model"));
    await user.type(screen.getByLabelText("Model"), "gpt-4o");

    expect(screen.queryByText("Settings saved")).not.toBeInTheDocument();
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
      expect(screen.getByText(/Validation failed/)).toBeInTheDocument();
    });
  });

  it("shows notes save location controls in settings mode", async () => {
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(pickNotesSaveDir).mockResolvedValue("/tmp/opennote-notes");
    vi.mocked(saveAuth).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(saveSettings).mockResolvedValue({
      ...baseSettings,
      onboarding_completed: true,
      notes_save_dir: "/tmp/opennote-notes",
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

    expect(screen.getByText("Notes save location")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Clear location" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Choose folder" }));

    await waitFor(() => {
      expect(pickNotesSaveDir).toHaveBeenCalled();
      expect(screen.getByDisplayValue("/tmp/opennote-notes")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(saveSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          notes_save_dir: "/tmp/opennote-notes",
        }),
      );
    });
  });

  it("clears API key immediately after confirmation", async () => {
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(saveAuth).mockResolvedValue({
      has_api_key: false,
      has_bilibili_cookie: false,
    });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    const user = userEvent.setup();
    render(
      <ConfigForm
        mode="settings"
        locale="en"
        initialSettings={{ ...baseSettings, onboarding_completed: true }}
        onSettingsSaved={vi.fn()}
      />,
    );

    const clearButton = await screen.findByRole("button", {
      name: "Clear API key",
    });
    await user.click(clearButton);

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalled();
      expect(saveAuth).toHaveBeenCalledWith({ clear_api_key: true });
      expect(screen.getByText("API key cleared")).toBeInTheDocument();
    });

    confirmSpy.mockRestore();
  });

  it("clears notes save location immediately without waiting for Save", async () => {
    const onSettingsSaved = vi.fn();
    vi.mocked(getAuthStatus).mockResolvedValue({
      has_api_key: true,
      has_bilibili_cookie: false,
    });
    vi.mocked(saveSettings).mockResolvedValue({
      ...baseSettings,
      onboarding_completed: true,
      notes_save_dir: null,
    });

    const user = userEvent.setup();
    render(
      <ConfigForm
        mode="settings"
        locale="en"
        initialSettings={{
          ...baseSettings,
          onboarding_completed: true,
          notes_save_dir: "/tmp/opennote-notes",
        }}
        onSettingsSaved={onSettingsSaved}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Clear location" }));

    await waitFor(() => {
      expect(saveSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          notes_save_dir: null,
          base_url: baseSettings.base_url,
          model: baseSettings.model,
        }),
      );
      expect(onSettingsSaved).toHaveBeenCalled();
      expect(screen.getByText("Save location cleared")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
  });
});
