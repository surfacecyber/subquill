export type UpdaterErrorKey =
  | "updaterErrorCheck"
  | "updaterErrorDownload"
  | "updaterErrorInstall"
  | "updaterErrorNetwork";

export type UpdaterPhase =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "installing"
  | "error"
  | "unconfigured";

export interface UpdaterState {
  phase: UpdaterPhase;
  availableVersion?: string;
  /** 0-100 when total size is known; null means indeterminate progress. */
  downloadProgress?: number | null;
  errorKey?: UpdaterErrorKey;
}

export interface UpdaterUpdateInfo {
  version: string;
  downloadAndInstall: (
    onEvent: (event: UpdaterDownloadEvent) => void,
  ) => Promise<void>;
}

export type UpdaterDownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

export interface UpdaterRuntime {
  checkForUpdate: () => Promise<UpdaterUpdateInfo | null>;
  relaunch: () => Promise<void>;
}

export function createInitialUpdaterState(): UpdaterState {
  return { phase: "idle" };
}

export function isUpdaterBusy(phase: UpdaterPhase): boolean {
  return (
    phase === "checking" || phase === "downloading" || phase === "installing"
  );
}

export function classifyUpdaterError(error: unknown): UpdaterErrorKey {
  const message =
    error instanceof Error
      ? error.message.toLowerCase()
      : String(error).toLowerCase();

  if (
    message.includes("network") ||
    message.includes("timeout") ||
    message.includes("connection")
  ) {
    return "updaterErrorNetwork";
  }

  if (message.includes("install")) {
    return "updaterErrorInstall";
  }

  if (message.includes("download")) {
    return "updaterErrorDownload";
  }

  return "updaterErrorCheck";
}

export function isUpdaterUnconfiguredError(error: unknown): boolean {
  const message =
    error instanceof Error
      ? error.message.toLowerCase()
      : String(error).toLowerCase();

  return (
    message.includes("pubkey") ||
    message.includes("public key") ||
    message.includes("endpoint") ||
    message.includes("not configured") ||
    message.includes("updater is not configured") ||
    message.includes("missing updater")
  );
}

export function reduceDownloadEvent(
  current: UpdaterState,
  event: UpdaterDownloadEvent,
  downloadedBytes: number,
  contentLength?: number,
): { state: UpdaterState; downloadedBytes: number; contentLength?: number } {
  switch (event.event) {
    case "Started": {
      const total = event.data.contentLength;
      return {
        state: {
          ...current,
          phase: "downloading",
          downloadProgress: total ? 0 : null,
        },
        downloadedBytes: 0,
        contentLength: total,
      };
    }
    case "Progress": {
      const nextDownloaded = downloadedBytes + event.data.chunkLength;
      const progress =
        contentLength && contentLength > 0
          ? Math.min(100, Math.round((nextDownloaded / contentLength) * 100))
          : null;
      return {
        state: {
          ...current,
          phase: "downloading",
          downloadProgress: progress,
        },
        downloadedBytes: nextDownloaded,
        contentLength,
      };
    }
    case "Finished":
      return {
        state: { ...current, phase: "installing", downloadProgress: 100 },
        downloadedBytes,
        contentLength,
      };
    default:
      return { state: current, downloadedBytes, contentLength };
  }
}

export interface UpdaterCheckResult {
  state: UpdaterState;
  update: UpdaterUpdateInfo | null;
}

export async function runUpdaterCheck(
  runtime: UpdaterRuntime,
  isDevBuild: boolean,
): Promise<UpdaterCheckResult> {
  if (isDevBuild) {
    return { state: { phase: "unconfigured" }, update: null };
  }

  try {
    const update = await runtime.checkForUpdate();
    if (!update) {
      return { state: { phase: "up-to-date" }, update: null };
    }

    return {
      state: {
        phase: "available",
        availableVersion: update.version,
      },
      update,
    };
  } catch (error) {
    if (isUpdaterUnconfiguredError(error)) {
      return { state: { phase: "unconfigured" }, update: null };
    }

    return {
      state: {
        phase: "error",
        errorKey: classifyUpdaterError(error),
      },
      update: null,
    };
  }
}

export async function runUpdaterInstall(
  runtime: UpdaterRuntime,
  update: UpdaterUpdateInfo,
  onProgress: (state: UpdaterState) => void,
  initialState: UpdaterState,
): Promise<UpdaterState> {
  let state: UpdaterState = {
    ...initialState,
    phase: "downloading",
    downloadProgress: null,
  };
  onProgress(state);

  let downloadedBytes = 0;
  let contentLength: number | undefined;

  try {
    await update.downloadAndInstall((event) => {
      const reduced = reduceDownloadEvent(state, event, downloadedBytes, contentLength);
      state = reduced.state;
      downloadedBytes = reduced.downloadedBytes;
      contentLength = reduced.contentLength;
      onProgress(state);
    });

    state = { ...state, phase: "installing" };
    onProgress(state);
    await runtime.relaunch();
    return state;
  } catch (error) {
    const nextState: UpdaterState = {
      phase: "error",
      errorKey: classifyUpdaterError(error),
    };
    onProgress(nextState);
    return nextState;
  }
}
