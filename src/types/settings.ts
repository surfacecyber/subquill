export type Locale = "zh" | "en" | "system";

export interface SettingsView {
  version: number;
  base_url: string;
  model: string;
  locale: Locale;
  onboarding_completed: boolean;
}

export interface SaveSettingsInput {
  base_url: string;
  model: string;
  locale: Locale;
  onboarding_completed: boolean;
}

export interface AuthStatus {
  has_api_key: boolean;
  has_bilibili_cookie: boolean;
}

export interface SaveAuthInput {
  api_key?: string;
  bilibili_cookie?: string;
  /** Explicitly remove a saved Bilibili cookie. Omit to preserve existing value. */
  clear_bilibili_cookie?: boolean;
}

export interface AppErrorPayload {
  code: string;
  message: string;
}
