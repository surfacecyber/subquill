import { describe, expect, it, vi } from "vitest";

import {
  classifyUpdaterError,
  isUpdaterBusy,
  isUpdaterUnconfiguredError,
  reduceDownloadEvent,
  runUpdaterCheck,
  runUpdaterInstall,
  type UpdaterRuntime,
} from "./updaterState";

describe("updaterState", () => {
  it("tracks busy phases", () => {
    expect(isUpdaterBusy("checking")).toBe(true);
    expect(isUpdaterBusy("downloading")).toBe(true);
    expect(isUpdaterBusy("available")).toBe(false);
  });

  it("reports no update when the runtime returns null", async () => {
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn().mockResolvedValue(null),
      relaunch: vi.fn(),
    };

    await expect(runUpdaterCheck(runtime, false)).resolves.toEqual({
      state: { phase: "up-to-date" },
      update: null,
    });
  });

  it("reports an available update", async () => {
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn().mockResolvedValue({
        version: "0.2.0",
        downloadAndInstall: vi.fn(),
      }),
      relaunch: vi.fn(),
    };

    await expect(runUpdaterCheck(runtime, false)).resolves.toEqual({
      state: { phase: "available", availableVersion: "0.2.0" },
      update: expect.objectContaining({ version: "0.2.0" }),
    });
  });

  it("marks dev builds as unconfigured", async () => {
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn(),
      relaunch: vi.fn(),
    };

    await expect(runUpdaterCheck(runtime, true)).resolves.toEqual({
      state: { phase: "unconfigured" },
      update: null,
    });
    expect(runtime.checkForUpdate).not.toHaveBeenCalled();
  });

  it("tracks download progress when content length is known", () => {
    const started = reduceDownloadEvent(
      { phase: "available", availableVersion: "0.2.0" },
      { event: "Started", data: { contentLength: 100 } },
      0,
    );
    expect(started.state.downloadProgress).toBe(0);

    const progressed = reduceDownloadEvent(
      started.state,
      { event: "Progress", data: { chunkLength: 25 } },
      started.downloadedBytes,
      started.contentLength,
    );
    expect(progressed.state.downloadProgress).toBe(25);
  });

  it("uses indeterminate progress when content length is missing", () => {
    const started = reduceDownloadEvent(
      { phase: "available", availableVersion: "0.2.0" },
      { event: "Started", data: {} },
      0,
    );
    expect(started.state.downloadProgress).toBeNull();
  });

  it("maps failures to localized error keys without remote details", () => {
    expect(classifyUpdaterError(new Error("network timeout"))).toBe(
      "updaterErrorNetwork",
    );
    expect(isUpdaterUnconfiguredError(new Error("missing updater pubkey"))).toBe(
      true,
    );
  });

  it("installs updates and relaunches", async () => {
    const downloadAndInstall = vi.fn(async (onEvent) => {
      onEvent({ event: "Started", data: { contentLength: 10 } });
      onEvent({ event: "Progress", data: { chunkLength: 10 } });
      onEvent({ event: "Finished" });
    });
    const relaunch = vi.fn().mockResolvedValue(undefined);
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn(),
      relaunch,
    };

    const progressStates: string[] = [];
    await runUpdaterInstall(
      runtime,
      { version: "0.2.0", downloadAndInstall },
      (state) => progressStates.push(state.phase),
      { phase: "available", availableVersion: "0.2.0" },
    );

    expect(downloadAndInstall).toHaveBeenCalled();
    expect(relaunch).toHaveBeenCalled();
    expect(progressStates).toContain("downloading");
    expect(progressStates).toContain("installing");
  });

  it("reports install failures", async () => {
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn(),
      relaunch: vi.fn(),
    };

    const result = await runUpdaterInstall(
      runtime,
      {
        version: "0.2.0",
        downloadAndInstall: vi.fn().mockRejectedValue(new Error("download failed")),
      },
      () => {},
      { phase: "available", availableVersion: "0.2.0" },
    );

    expect(result.phase).toBe("error");
    expect(result.errorKey).toBe("updaterErrorDownload");
  });
});
