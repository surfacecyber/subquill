import { invoke } from "@tauri-apps/api/core";

import type {
  AppErrorPayload,
  AuthStatus,
  SaveAuthInput,
  SaveSettingsInput,
  SettingsView,
} from "../types/settings";
import type { BilibiliSubtitleResult } from "../types/bilibili";

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

export async function getSettings(): Promise<SettingsView> {
  try {
    return await invoke<SettingsView>("get_settings");
  } catch (error) {
    throw toAppError(error);
  }
}

export async function saveSettings(
  input: SaveSettingsInput,
): Promise<SettingsView> {
  try {
    return await invoke<SettingsView>("save_settings", { input });
  } catch (error) {
    throw toAppError(error);
  }
}

export async function pickNotesSaveDir(): Promise<string | null> {
  try {
    return await invoke<string | null>("pick_notes_save_dir");
  } catch (error) {
    throw toAppError(error);
  }
}

export async function getAuthStatus(): Promise<AuthStatus> {
  try {
    return await invoke<AuthStatus>("get_auth_status");
  } catch (error) {
    throw toAppError(error);
  }
}

export async function saveAuth(input: SaveAuthInput): Promise<AuthStatus> {
  try {
    return await invoke<AuthStatus>("save_auth", { input });
  } catch (error) {
    throw toAppError(error);
  }
}

export async function fetchBilibiliSubtitles(
  url: string,
): Promise<BilibiliSubtitleResult> {
  try {
    return await invoke<BilibiliSubtitleResult>("fetch_bilibili_subtitles", {
      url,
    });
  } catch (error) {
    throw toAppError(error);
  }
}

export interface AppInfo {
  name: string;
  version: string;
}

export async function getAppInfo(): Promise<AppInfo> {
  try {
    return await invoke<AppInfo>("get_app_info");
  } catch (error) {
    throw toAppError(error);
  }
}
