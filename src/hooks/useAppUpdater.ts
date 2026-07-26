import { useCallback, useEffect, useRef, useState } from "react";

import {
  createInitialUpdaterState,
  isUpdaterBusy,
  runUpdaterCheck,
  runUpdaterInstall,
  type UpdaterRuntime,
  type UpdaterState,
  type UpdaterUpdateInfo,
} from "../updater/updaterState";

interface UseAppUpdaterOptions {
  autoCheck?: boolean;
  isDevBuild?: boolean;
  runtime?: UpdaterRuntime;
}

export interface AppUpdaterController {
  state: UpdaterState;
  isBusy: boolean;
  checkForUpdates: () => Promise<UpdaterState>;
  installUpdate: () => Promise<void>;
  dismissAvailable: () => void;
}

async function defaultCheckForUpdate(): Promise<UpdaterUpdateInfo | null> {
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) {
    return null;
  }

  return {
    version: update.version,
    downloadAndInstall: (onEvent) =>
      update.downloadAndInstall((event) => {
        onEvent(event);
      }),
  };
}

async function defaultRelaunch(): Promise<void> {
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}

const defaultRuntime: UpdaterRuntime = {
  checkForUpdate: defaultCheckForUpdate,
  relaunch: defaultRelaunch,
};

type UpdaterOperation = "idle" | "checking" | "installing";

export function useAppUpdater({
  autoCheck = false,
  isDevBuild = import.meta.env.DEV,
  runtime = defaultRuntime,
}: UseAppUpdaterOptions = {}): AppUpdaterController {
  const [state, setState] = useState<UpdaterState>(createInitialUpdaterState);
  const pendingUpdateRef = useRef<UpdaterUpdateInfo | null>(null);
  const autoCheckStartedRef = useRef(false);
  const mountedRef = useRef(true);
  const operationRef = useRef<UpdaterOperation>("idle");
  const stateRef = useRef(state);

  stateRef.current = state;

  const safeSetState = useCallback((next: UpdaterState) => {
    if (mountedRef.current) {
      setState(next);
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const checkForUpdates = useCallback(async () => {
    if (operationRef.current !== "idle") {
      return stateRef.current;
    }

    operationRef.current = "checking";
    safeSetState({ phase: "checking" });

    try {
      const result = await runUpdaterCheck(runtime, isDevBuild);
      pendingUpdateRef.current = result.update;
      safeSetState(result.state);
      return result.state;
    } finally {
      if (operationRef.current === "checking") {
        operationRef.current = "idle";
      }
    }
  }, [isDevBuild, runtime, safeSetState]);

  const installUpdate = useCallback(async () => {
    if (operationRef.current !== "idle") {
      return;
    }

    const update = pendingUpdateRef.current;
    if (!update) {
      safeSetState({ phase: "error", errorKey: "updaterErrorInstall" });
      return;
    }

    operationRef.current = "installing";
    const initialState: UpdaterState = {
      ...stateRef.current,
      phase: "available",
      availableVersion: update.version,
    };

    try {
      await runUpdaterInstall(runtime, update, safeSetState, initialState);
    } finally {
      if (operationRef.current === "installing") {
        operationRef.current = "idle";
      }
    }
  }, [runtime, safeSetState]);

  const dismissAvailable = useCallback(() => {
    if (operationRef.current !== "idle") {
      return;
    }

    pendingUpdateRef.current = null;
    safeSetState({ phase: "idle" });
  }, [safeSetState]);

  useEffect(() => {
    if (!autoCheck || isDevBuild || autoCheckStartedRef.current) {
      return undefined;
    }

    autoCheckStartedRef.current = true;
    let cancelled = false;

    void (async () => {
      if (operationRef.current !== "idle") {
        return;
      }

      operationRef.current = "checking";

      try {
        const result = await runUpdaterCheck(runtime, isDevBuild);
        if (!cancelled && mountedRef.current) {
          pendingUpdateRef.current = result.update;
          safeSetState(result.state);
        }
      } finally {
        if (operationRef.current === "checking") {
          operationRef.current = "idle";
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [autoCheck, isDevBuild, runtime, safeSetState]);

  return {
    state,
    isBusy: isUpdaterBusy(state.phase),
    checkForUpdates,
    installUpdate,
    dismissAvailable,
  };
}
