import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { UpdateBanner } from "../components/UpdateBanner";
import { useAppUpdater } from "../hooks/useAppUpdater";
import { AboutPage } from "../pages/AboutPage";
import type { UpdaterRuntime } from "./updaterState";

vi.mock("../api/commands", () => ({
  getAppInfo: vi.fn().mockResolvedValue({ name: "OpenNote", version: "0.1.0" }),
}));

function SharedUpdaterShell({ runtime }: { runtime: UpdaterRuntime }) {
  const updater = useAppUpdater({
    autoCheck: true,
    isDevBuild: false,
    runtime,
  });

  return (
    <>
      <UpdateBanner
        locale="en"
        state={updater.state}
        onInstall={() => {
          void updater.installUpdate();
        }}
        onDismiss={updater.dismissAvailable}
      />
      <AboutPage locale="en" updater={updater} />
    </>
  );
}

describe("shared updater controller", () => {
  it("shows startup auto-check version on About without a second runtime", async () => {
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn().mockResolvedValue({
        version: "0.9.0",
        downloadAndInstall: vi.fn(),
      }),
      relaunch: vi.fn(),
    };

    render(<SharedUpdaterShell runtime={runtime} />);

    await waitFor(() => {
      expect(
        screen.getByText("Version 0.9.0 is available. Install now?"),
      ).toBeInTheDocument();
    });

    expect(screen.getByText("Update available: 0.9.0")).toBeInTheDocument();
    expect(runtime.checkForUpdate).toHaveBeenCalledTimes(1);
  });

  it("hides the update banner while deferred", async () => {
    const runtime: UpdaterRuntime = {
      checkForUpdate: vi.fn().mockResolvedValue({
        version: "0.9.0",
        downloadAndInstall: vi.fn(),
      }),
      relaunch: vi.fn(),
    };

    function DeferredShell() {
      const updater = useAppUpdater({
        autoCheck: true,
        isDevBuild: false,
        runtime,
      });
      return (
        <UpdateBanner
          locale="en"
          state={updater.state}
          deferred
          onInstall={() => undefined}
          onDismiss={updater.dismissAvailable}
        />
      );
    }

    render(<DeferredShell />);

    await waitFor(() => {
      expect(runtime.checkForUpdate).toHaveBeenCalled();
    });

    expect(
      screen.queryByText("Version 0.9.0 is available. Install now?"),
    ).not.toBeInTheDocument();
  });
});
