import { ConfigForm } from "../components/ConfigForm";
import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import type { SettingsView } from "../types/settings";

interface OnboardingPageProps {
  locale: UiLocale;
  initialSettings: SettingsView;
  onCompleted: (settings: SettingsView) => void;
}

export function OnboardingPage({
  locale,
  initialSettings,
  onCompleted,
}: OnboardingPageProps) {
  return (
    <section className="onboarding" aria-labelledby="onboarding-heading">
      <header className="page-header">
        <p className="eyebrow">{t(locale, "appTitle")}</p>
        <h1 id="onboarding-heading">{t(locale, "setupTitle")}</h1>
        <p className="muted">{t(locale, "setupSubtitle")}</p>
      </header>

      <ConfigForm
        mode="onboarding"
        locale={locale}
        initialSettings={initialSettings}
        onSettingsSaved={onCompleted}
      />
    </section>
  );
}
