import type { FormEvent } from "react";
import { useEffect, useState } from "react";

import {
  getAuthStatus,
  pickNotesSaveDir,
  saveAuth,
  saveSettings,
} from "../api/commands";
import { testLlm } from "../api/job";
import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import { localizeError } from "../i18n/errors";
import { useNotify } from "../notifications/NotificationProvider";
import type { AppErrorPayload, AuthStatus, Locale, SettingsView } from "../types/settings";

export type ConfigFormMode = "onboarding" | "settings";

interface ConfigFormProps {
  mode: ConfigFormMode;
  locale: UiLocale;
  initialSettings: SettingsView;
  onSettingsSaved: (settings: SettingsView) => void;
  onDirtyChange?: (dirty: boolean) => void;
}

export function ConfigForm({
  mode,
  locale,
  initialSettings,
  onSettingsSaved,
  onDirtyChange,
}: ConfigFormProps) {
  const { notify, clear: clearNotifications } = useNotify();
  const [baseUrl, setBaseUrl] = useState(initialSettings.base_url);
  const [model, setModel] = useState(initialSettings.model);
  const [settingsLocale, setSettingsLocale] = useState<Locale>(
    initialSettings.locale,
  );
  const [notesSaveDir, setNotesSaveDir] = useState<string | null>(
    initialSettings.notes_save_dir,
  );
  const [apiKey, setApiKey] = useState("");
  const [bilibiliCookie, setBilibiliCookie] = useState("");
  const [authStatus, setAuthStatus] = useState<AuthStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [errorDetail, setErrorDetail] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [clearingNotesDir, setClearingNotesDir] = useState(false);

  const dirty =
    baseUrl !== initialSettings.base_url ||
    model !== initialSettings.model ||
    settingsLocale !== initialSettings.locale ||
    notesSaveDir !== initialSettings.notes_save_dir ||
    apiKey.trim().length > 0 ||
    bilibiliCookie.trim().length > 0;

  useEffect(() => {
    onDirtyChange?.(dirty);
  }, [dirty, onDirtyChange]);

  useEffect(() => {
    void getAuthStatus()
      .then(setAuthStatus)
      .catch((err) => {
        setAuthStatus({ has_api_key: false, has_bilibili_cookie: false });
        const payload = err as AppErrorPayload;
        const localized = localizeError(locale, payload.code, payload.message);
        setError(t(locale, "errorAuthLoadFailed"));
        setErrorDetail(localized.detail ?? localized.title);
      });
  }, [locale]);

  function showError(payload: AppErrorPayload) {
    const localized = localizeError(locale, payload.code, payload.message);
    setError(localized.title);
    setErrorDetail(localized.detail ?? null);
  }

  function showContextualError(
    contextKey: "errorAuthSaveFailed" | "errorSettingsSaveFailed",
    payload: AppErrorPayload,
  ) {
    const localized = localizeError(locale, payload.code, payload.message);
    setError(t(locale, contextKey));
    setErrorDetail(
      localized.detail
        ? `${localized.title} — ${localized.detail}`
        : localized.title,
    );
  }

  /** Clear stale toast feedback once the form is dirty again. */
  function markDirty() {
    clearNotifications();
  }

  function markConnectionDirty() {
    markDirty();
  }

  async function handleTestConnection() {
    setTesting(true);
    clearNotifications();
    setError(null);
    setErrorDetail(null);

    try {
      const result = await testLlm({
        base_url: baseUrl,
        model,
        api_key: apiKey.trim().length > 0 ? apiKey.trim() : undefined,
      });
      notify(
        "success",
        `${t(locale, "testConnectionSuccess")} — ${t(locale, "testConnectionModel")}: ${result.model}`,
      );
    } catch (err) {
      showError(err as AppErrorPayload);
    } finally {
      setTesting(false);
    }
  }

  async function handleClearApiKey() {
    if (!window.confirm(t(locale, "clearApiKeyConfirm"))) {
      return;
    }

    clearNotifications();
    setError(null);
    setErrorDetail(null);

    try {
      const status = await saveAuth({ clear_api_key: true });
      setAuthStatus(status);
      setApiKey("");
      notify("success", t(locale, "apiKeyCleared"));
    } catch (err) {
      showError(err as AppErrorPayload);
    }
  }

  async function handleClearCookie() {
    if (!window.confirm(t(locale, "clearCookieConfirm"))) {
      return;
    }

    clearNotifications();
    setError(null);
    setErrorDetail(null);

    try {
      const status = await saveAuth({ clear_bilibili_cookie: true });
      setAuthStatus(status);
      setBilibiliCookie("");
      notify("success", t(locale, "cookieCleared"));
    } catch (err) {
      showError(err as AppErrorPayload);
    }
  }

  async function handlePickNotesSaveDir() {
    setError(null);
    setErrorDetail(null);
    markDirty();

    try {
      const picked = await pickNotesSaveDir();
      if (picked) {
        setNotesSaveDir(picked);
      }
    } catch (err) {
      showError(err as AppErrorPayload);
    }
  }

  async function handleClearNotesSaveDir() {
    if (!notesSaveDir) {
      return;
    }

    setClearingNotesDir(true);
    setError(null);
    setErrorDetail(null);
    clearNotifications();

    try {
      // Persist only the path clear; leave other unsaved form edits alone.
      const settings = await saveSettings({
        base_url: initialSettings.base_url,
        model: initialSettings.model,
        locale: initialSettings.locale,
        onboarding_completed: initialSettings.onboarding_completed,
        notes_save_dir: null,
      });
      setNotesSaveDir(null);
      notify("success", t(locale, "notesSaveDirCleared"));
      onSettingsSaved(settings);
    } catch (err) {
      showError(err as AppErrorPayload);
    } finally {
      setClearingNotesDir(false);
    }
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setErrorDetail(null);
    clearNotifications();
    setSaving(true);

    let authSaved = false;

    try {
      if (!authStatus?.has_api_key && apiKey.trim().length === 0) {
        setError(t(locale, "requiredApiKey"));
        return;
      }

      try {
        await saveAuth({
          api_key: apiKey.trim().length > 0 ? apiKey.trim() : undefined,
          bilibili_cookie:
            bilibiliCookie.trim().length > 0
              ? bilibiliCookie.trim()
              : undefined,
        });
        authSaved = true;
      } catch (err) {
        showContextualError("errorAuthSaveFailed", err as AppErrorPayload);
        return;
      }

      try {
        const settings = await saveSettings({
          base_url: baseUrl,
          model,
          locale: settingsLocale,
          onboarding_completed:
            mode === "onboarding" ? true : initialSettings.onboarding_completed,
          notes_save_dir:
            mode === "settings"
              ? notesSaveDir
              : initialSettings.notes_save_dir,
        });

        const nextAuthStatus = await getAuthStatus();
        setAuthStatus(nextAuthStatus);
        setApiKey("");
        setBilibiliCookie("");
        setNotesSaveDir(settings.notes_save_dir);
        notify("success", t(locale, "saved"));
        onSettingsSaved(settings);
      } catch (err) {
        if (authSaved) {
          showContextualError(
            "errorSettingsSaveFailed",
            err as AppErrorPayload,
          );
        } else {
          showError(err as AppErrorPayload);
        }
        return;
      }
    } finally {
      setSaving(false);
    }
  }

  const submitLabel =
    mode === "onboarding"
      ? saving
        ? t(locale, "saving")
        : t(locale, "saveAndContinue")
      : saving
        ? t(locale, "saving")
        : t(locale, "save");

  return (
    <form
      className={`config-form${mode === "settings" ? " config-form-docked" : ""}`}
      onSubmit={handleSubmit}
    >
      <div className="config-form-body">
      <label>
        <span>{t(locale, "baseUrl")}</span>
        <input
          type="url"
          name="base_url"
          value={baseUrl}
          onChange={(event) => {
            markConnectionDirty();
            setBaseUrl(event.target.value);
          }}
          placeholder="https://api.openai.com/v1"
          required
          autoComplete="off"
          spellCheck={false}
        />
      </label>

      <label>
        <span>{t(locale, "model")}</span>
        <input
          type="text"
          name="model"
          value={model}
          onChange={(event) => {
            markConnectionDirty();
            setModel(event.target.value);
          }}
          placeholder="deepseek-v4-flash"
          required
          autoComplete="off"
          spellCheck={false}
        />
      </label>

      <label>
        <span>{t(locale, "apiKey")}</span>
        <input
          type="password"
          name="api_key"
          value={apiKey}
          onChange={(event) => {
            markConnectionDirty();
            setApiKey(event.target.value);
          }}
          placeholder={
            authStatus?.has_api_key ? t(locale, "apiKeySaved") : ""
          }
          autoComplete="off"
          spellCheck={false}
        />
      </label>

      <div className="form-row">
        <button
          type="button"
          className="btn-secondary"
          onClick={() => void handleTestConnection()}
          disabled={testing || saving}
        >
          {testing ? t(locale, "testingConnection") : t(locale, "testConnection")}
        </button>
        {mode === "settings" && authStatus?.has_api_key ? (
          <button
            type="button"
            className="btn-secondary"
            onClick={() => void handleClearApiKey()}
            disabled={testing || saving}
          >
            {t(locale, "clearApiKey")}
          </button>
        ) : null}
      </div>

      <label>
        <span>{t(locale, "bilibiliCookie")}</span>
        <input
          type="password"
          name="bilibili_cookie"
          value={bilibiliCookie}
          onChange={(event) => {
            markDirty();
            setBilibiliCookie(event.target.value);
          }}
          placeholder={
            authStatus?.has_bilibili_cookie
              ? t(locale, "bilibiliCookieSaved")
              : t(locale, "bilibiliCookiePlaceholder")
          }
          autoComplete="off"
          spellCheck={false}
        />
        <details className="cookie-hint">
          <summary>{t(locale, "bilibiliCookieHowTo")}</summary>
          <span className="muted-inline">{t(locale, "bilibiliCookieHint")}</span>
        </details>
      </label>

      {mode === "settings" && authStatus?.has_bilibili_cookie ? (
        <div className="form-row">
          <button
            type="button"
            className="btn-secondary"
            onClick={() => void handleClearCookie()}
          >
            {t(locale, "clearCookie")}
          </button>
        </div>
      ) : null}

      <label>
        <span>{t(locale, "locale")}</span>
        <select
          name="locale"
          value={settingsLocale}
          onChange={(event) => {
            markDirty();
            setSettingsLocale(event.target.value as Locale);
          }}
        >
          <option value="system">{t(locale, "localeSystem")}</option>
          <option value="zh">{t(locale, "localeZh")}</option>
          <option value="en">{t(locale, "localeEn")}</option>
        </select>
      </label>

      {mode === "settings" ? (
        <div className="notes-save-dir">
          <label>
            <span>{t(locale, "notesSaveDir")}</span>
            <input
              type="text"
              name="notes_save_dir"
              value={notesSaveDir ?? ""}
              readOnly
              placeholder={t(locale, "notesSaveDirPlaceholder")}
              autoComplete="off"
              spellCheck={false}
            />
            <span className="muted-inline">{t(locale, "notesSaveDirHint")}</span>
          </label>
          <div className="form-row">
            <button
              type="button"
              className="btn-secondary"
              onClick={() => void handlePickNotesSaveDir()}
              disabled={saving || clearingNotesDir}
            >
              {t(locale, "pickNotesSaveDir")}
            </button>
            <button
              type="button"
              className="btn-secondary"
              onClick={() => void handleClearNotesSaveDir()}
              disabled={saving || clearingNotesDir || !notesSaveDir}
            >
              {t(locale, "clearNotesSaveDir")}
            </button>
          </div>
        </div>
      ) : null}

      {error ? (
        <div className="form-error-block" role="alert">
          <p className="form-error">{error}</p>
          {errorDetail ? <p className="form-error-detail">{errorDetail}</p> : null}
        </div>
      ) : null}
      </div>

      <div className="form-actions">
        <button type="submit" className="btn-primary" disabled={saving}>
          {submitLabel}
        </button>
      </div>
    </form>
  );
}
