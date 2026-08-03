import { useCallback, useEffect, useState } from "react";

import { getSettings } from "./api/commands";
import { UpdateBanner } from "./components/UpdateBanner";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { resolveUiLocale, t } from "./i18n";
import { AboutPage } from "./pages/AboutPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { SettingsPage } from "./pages/SettingsPage";
import { WorkspacePage } from "./pages/WorkspacePage";
import type { SettingsView } from "./types/settings";
import "./App.css";

type AppView = "workspace" | "settings" | "about";

function App() {
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [view, setView] = useState<AppView>("workspace");
  const [settingsDirty, setSettingsDirty] = useState(false);
  const [jobActive, setJobActive] = useState(false);

  useEffect(() => {
    void getSettings()
      .then(setSettings)
      .catch(() => setLoadError(true))
      .finally(() => setLoading(false));
  }, []);

  const uiLocale = settings ? resolveUiLocale(settings.locale) : "en";
  const updater = useAppUpdater({ autoCheck: true });

  useEffect(() => {
    document.documentElement.lang = uiLocale === "zh" ? "zh-CN" : "en";
  }, [uiLocale]);

  const navigate = useCallback(
    (next: AppView) => {
      if (view === "settings" && next !== "settings" && settingsDirty) {
        if (!window.confirm(t(uiLocale, "unsavedSettingsConfirm"))) {
          return;
        }
        setSettingsDirty(false);
      }
      setView(next);
    },
    [view, settingsDirty, uiLocale],
  );

  const handleSettingsSaved = useCallback((next: SettingsView) => {
    setSettings(next);
    setSettingsDirty(false);
  }, []);

  if (loading) {
    return (
      <main className="app-shell">
        <p className="muted">{t(uiLocale, "loading")}</p>
      </main>
    );
  }

  if (loadError || !settings) {
    return (
      <main className="app-shell">
        <p className="form-error" role="alert">
          {t(uiLocale, "loadSettingsFailed")}
        </p>
      </main>
    );
  }

  if (!settings.onboarding_completed) {
    return (
      <main className="app-shell onboarding-shell">
        <OnboardingPage
          locale={uiLocale}
          initialSettings={settings}
          onCompleted={setSettings}
        />
      </main>
    );
  }

  return (
    <div className="app-layout">
      <div className="titlebar-drag" data-tauri-drag-region aria-hidden="true" />
      <nav className="app-nav" aria-label={t(uiLocale, "navAriaLabel")}>
        <p className="nav-brand">{t(uiLocale, "appTitle")}</p>
        <ul className="nav-list">
          <li>
            <button
              type="button"
              className={`nav-link${view === "workspace" ? " nav-link-active" : ""}`}
              aria-current={view === "workspace" ? "page" : undefined}
              onClick={() => navigate("workspace")}
            >
              {t(uiLocale, "navWorkspace")}
            </button>
          </li>
          <li>
            <button
              type="button"
              className={`nav-link${view === "settings" ? " nav-link-active" : ""}`}
              aria-current={view === "settings" ? "page" : undefined}
              onClick={() => navigate("settings")}
            >
              {t(uiLocale, "navSettings")}
            </button>
          </li>
          <li>
            <button
              type="button"
              className={`nav-link${view === "about" ? " nav-link-active" : ""}`}
              aria-current={view === "about" ? "page" : undefined}
              onClick={() => navigate("about")}
            >
              {t(uiLocale, "navAbout")}
            </button>
          </li>
        </ul>
      </nav>

      <main className="app-main">
        <UpdateBanner
          locale={uiLocale}
          state={updater.state}
          deferred={jobActive}
          onInstall={() => {
            void updater.installUpdate();
          }}
          onDismiss={updater.dismissAvailable}
        />
        <div
          className="view-panel"
          hidden={view !== "workspace"}
          aria-hidden={view !== "workspace"}
          {...(view !== "workspace" ? { inert: true } : {})}
        >
          <WorkspacePage
            locale={uiLocale}
            notesSaveDir={settings.notes_save_dir}
            onOpenSettings={() => navigate("settings")}
            onJobActiveChange={setJobActive}
          />
        </div>
        {view === "settings" ? (
          <SettingsPage
            locale={uiLocale}
            settings={settings}
            onSettingsSaved={handleSettingsSaved}
            onDirtyChange={setSettingsDirty}
          />
        ) : null}
        {view === "about" ? (
          <AboutPage locale={uiLocale} updater={updater} />
        ) : null}
      </main>
    </div>
  );
}

export default App;
