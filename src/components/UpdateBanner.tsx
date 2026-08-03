import type { UiLocale } from "../i18n";
import { t } from "../i18n";
import type { UpdaterState } from "../updater/updaterState";

interface UpdateBannerProps {
  locale: UiLocale;
  state: UpdaterState;
  /** When true (e.g. note job running), hide the banner to reduce distraction. */
  deferred?: boolean;
  onInstall: () => void;
  onDismiss: () => void;
}

export function UpdateBanner({
  locale,
  state,
  deferred = false,
  onInstall,
  onDismiss,
}: UpdateBannerProps) {
  if (deferred || state.phase !== "available" || !state.availableVersion) {
    return null;
  }

  return (
    <aside className="update-banner" role="status" aria-live="polite">
      <p className="update-banner-text">
        {t(locale, "updaterBannerAvailable", {
          version: state.availableVersion,
        })}
      </p>
      <div className="update-banner-actions">
        <button type="button" className="btn-primary" onClick={onInstall}>
          {t(locale, "updaterInstall")}
        </button>
        <button type="button" className="btn-secondary" onClick={onDismiss}>
          {t(locale, "updaterDismiss")}
        </button>
      </div>
    </aside>
  );
}

export function updaterStatusLabel(
  locale: UiLocale,
  state: UpdaterState,
): string {
  switch (state.phase) {
    case "idle":
      return t(locale, "updaterStatusIdle");
    case "checking":
      return t(locale, "updaterStatusChecking");
    case "up-to-date":
      return t(locale, "updaterStatusUpToDate");
    case "available":
      return t(locale, "updaterStatusAvailable", {
        version: state.availableVersion ?? "",
      });
    case "downloading":
      return state.downloadProgress == null
        ? t(locale, "updaterStatusDownloadingIndeterminate")
        : t(locale, "updaterStatusDownloading", {
            progress: state.downloadProgress,
          });
    case "installing":
      return t(locale, "updaterStatusInstalling");
    case "unconfigured":
      return t(locale, "updaterStatusUnconfigured");
    case "error":
      return state.errorKey
        ? t(locale, state.errorKey)
        : t(locale, "updaterErrorCheck");
    default:
      return t(locale, "updaterStatusIdle");
  }
}
