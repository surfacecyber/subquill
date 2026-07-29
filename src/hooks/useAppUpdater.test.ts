import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useAppUpdater } from "./useAppUpdater";
import type { UpdaterRuntime, UpdaterUpdateInfo } from "../updater/updaterState";

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

function mockUpdate(version: string): UpdaterUpdateInfo {
  return {
    version,
    downloadAndInstall: vi.fn(async () => {}),
  };
}

describe("useAppUpdater", () => {
  it("rejects parallel check calls while a check is in flight", async () => {
    const deferred = createDeferred<UpdaterUpdateInfo | null>();
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn(() => deferred.promise),
      relaunch: vi.fn(),
    };

    const { result } = renderHook(() =>
      useAppUpdater({ isDevBuild: false, runtime }),
    );

    let first!: Promise<unknown>;
    let second!: Promise<unknown>;
    act(() => {
      first = result.current.checkForUpdates();
      second = result.current.checkForUpdates();
    });

    expect(runtime.checkForUpdate).toHaveBeenCalledTimes(1);
    expect(result.current.state.phase).toBe("checking");

    await act(async () => {
      deferred.resolve(mockUpdate("0.2.0"));
      await first;
      await second;
    });

    expect(result.current.state.phase).toBe("available");
    expect(result.current.state.availableVersion).toBe("0.2.0");
  });

  it("rejects parallel install calls while installation is in progress", async () => {
    const installDeferred = createDeferred<void>();
    const update = {
      version: "0.2.0",
      downloadAndInstall: vi.fn(() => installDeferred.promise),
    };
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn().mockResolvedValue(update),
      relaunch: vi.fn(),
    };

    const { result } = renderHook(() =>
      useAppUpdater({ isDevBuild: false, runtime }),
    );

    await act(async () => {
      await result.current.checkForUpdates();
    });

    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      first = result.current.installUpdate();
      second = result.current.installUpdate();
    });

    expect(update.downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(result.current.isBusy).toBe(true);

    await act(async () => {
      installDeferred.resolve();
      await first;
      await second;
    });

    expect(runtime.relaunch).toHaveBeenCalledTimes(1);
  });

  it("does not set state after unmount when async check completes", async () => {
    const deferred = createDeferred<UpdaterUpdateInfo | null>();
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn(() => deferred.promise),
      relaunch: vi.fn(),
    };

    const { result, unmount } = renderHook(() =>
      useAppUpdater({ isDevBuild: false, runtime }),
    );

    act(() => {
      void result.current.checkForUpdates();
    });

    unmount();

    await act(async () => {
      deferred.resolve(mockUpdate("0.2.0"));
    });

    expect(runtime.checkForUpdate).toHaveBeenCalledTimes(1);
  });

  it("runs auto-check once on mount and exposes the result to consumers", async () => {
    const deferred = createDeferred<UpdaterUpdateInfo | null>();
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn(() => deferred.promise),
      relaunch: vi.fn(),
    };

    const { result } = renderHook(() =>
      useAppUpdater({ autoCheck: true, isDevBuild: false, runtime }),
    );

    await waitFor(() => {
      expect(result.current.state.phase).toBe("checking");
    });

    await act(async () => {
      deferred.resolve(mockUpdate("1.2.3"));
    });

    await waitFor(() => {
      expect(result.current.state.phase).toBe("available");
    });

    expect(runtime.checkForUpdate).toHaveBeenCalledTimes(1);
    expect(result.current.state.availableVersion).toBe("1.2.3");
  });
});
