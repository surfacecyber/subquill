import { useEffect, useState } from "react";

import { getAppInfo } from "../api/commands";
import { updaterStatusLabel } from "../components/UpdateBanner";
import type { AppUpdaterController } from "../hooks/useAppUpdater";
import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import pkg from "../../package.json";

const FALLBACK_VERSION = pkg.version;

interface AboutPageProps {
  locale: UiLocale;
  updater: AppUpdaterController;
}

export function AboutPage({ locale, updater }: AboutPageProps) {
  const [version, setVersion] = useState(FALLBACK_VERSION);
  const { state, isBusy, checkForUpdates, installUpdate } = updater;

  useEffect(() => {
    void getAppInfo()
      .then((info) => setVersion(info.version))
      .catch(() => {
        setVersion(FALLBACK_VERSION);
      });
  }, []);

  return (
    <section className="about-page" aria-labelledby="about-heading">
      <header className="page-header">
        <h1 id="about-heading">{t(locale, "aboutTitle")}</h1>
        <p className="muted">{t(locale, "aboutDescription")}</p>
      </header>

      <dl className="about-meta">
        <div className="about-row">
          <dt>{t(locale, "appTitle")}</dt>
          <dd>OpenNote</dd>
        </div>
        <div className="about-row">
          <dt>{t(locale, "version")}</dt>
          <dd>{version}</dd>
        </div>
        <div className="about-row">
          <dt>{t(locale, "updaterStatus")}</dt>
          <dd aria-live="polite">{updaterStatusLabel(locale, state)}</dd>
        </div>
      </dl>

      <div className="about-actions">
        <button
          type="button"
          className="btn-secondary"
          disabled={isBusy}
          onClick={() => {
            void checkForUpdates();
          }}
        >
          {t(locale, "updaterCheckNow")}
        </button>
        {state.phase === "available" ? (
          <button
            type="button"
            className="btn-primary"
            disabled={isBusy}
            onClick={() => {
              void installUpdate();
            }}
          >
            {t(locale, "updaterInstall")}
          </button>
        ) : null}
      </div>
    </section>
  );
}
