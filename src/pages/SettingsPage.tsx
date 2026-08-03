import { ConfigForm } from "../components/ConfigForm";
import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import type { SettingsView } from "../types/settings";

interface SettingsPageProps {
  locale: UiLocale;
  settings: SettingsView;
  onSettingsSaved: (settings: SettingsView) => void;
  onDirtyChange?: (dirty: boolean) => void;
}

export function SettingsPage({
  locale,
  settings,
  onSettingsSaved,
  onDirtyChange,
}: SettingsPageProps) {
  return (
    <section className="settings-page" aria-labelledby="settings-heading">
      <header className="page-header">
        <h1 id="settings-heading">{t(locale, "settingsTitle")}</h1>
        <p className="muted">{t(locale, "settingsSubtitle")}</p>
      </header>

      <ConfigForm
        mode="settings"
        locale={locale}
        initialSettings={settings}
        onSettingsSaved={onSettingsSaved}
        onDirtyChange={onDirtyChange}
      />
    </section>
  );
}
