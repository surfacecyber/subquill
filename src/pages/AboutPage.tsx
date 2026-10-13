import { useEffect, useState } from "react";

import { getAppInfo } from "../api/commands";
import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import pkg from "../../package.json";

const FALLBACK_VERSION = pkg.version;
const REPO_URL = "https://github.com/xuedongxue/subquill";

interface AboutPageProps {
  locale: UiLocale;
}

export function AboutPage({ locale }: AboutPageProps) {
  const [version, setVersion] = useState(FALLBACK_VERSION);

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
          <dd translate="no">Subquill</dd>
        </div>
        <div className="about-row">
          <dt>{t(locale, "version")}</dt>
          <dd>{version}</dd>
        </div>
      </dl>

      <div className="about-sections">
        <section className="about-section" aria-labelledby="about-platforms">
          <h2 id="about-platforms">{t(locale, "aboutPlatformsTitle")}</h2>
          <p className="muted">{t(locale, "aboutPlatforms")}</p>
        </section>
        <section className="about-section" aria-labelledby="about-updates">
          <h2 id="about-updates">{t(locale, "aboutUpdatesTitle")}</h2>
          <p className="muted">{t(locale, "aboutUpdates")}</p>
        </section>
        <section className="about-section" aria-labelledby="about-privacy">
          <h2 id="about-privacy">{t(locale, "aboutPrivacyTitle")}</h2>
          <p className="muted">{t(locale, "aboutPrivacy")}</p>
        </section>
        <section className="about-section" aria-labelledby="about-cookie">
          <h2 id="about-cookie">{t(locale, "aboutCookieTitle")}</h2>
          <p className="muted">{t(locale, "aboutCookie")}</p>
        </section>
      </div>

      <div className="about-actions">
        <a
          className="btn-secondary"
          href={REPO_URL}
          target="_blank"
          rel="noreferrer"
        >
          {t(locale, "aboutSourceRepo")}
        </a>
      </div>
    </section>
  );
}
