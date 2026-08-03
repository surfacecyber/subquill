import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  NotificationProvider,
  useNotify,
} from "./NotificationProvider";

function Probe({ message = "Saved ok" }: { message?: string }) {
  const { notify } = useNotify();
  return (
    <button type="button" onClick={() => notify("success", message)}>
      Trigger
    </button>
  );
}

describe("NotificationProvider", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows a toast that can be dismissed", async () => {
    const user = userEvent.setup();
    render(
      <NotificationProvider>
        <Probe />
      </NotificationProvider>,
    );

    await user.click(screen.getByRole("button", { name: "Trigger" }));
    expect(screen.getByText("Saved ok")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByText("Saved ok")).not.toBeInTheDocument();
  });

  it("auto-dismisses after timeout", () => {
    vi.useFakeTimers();
    render(
      <NotificationProvider>
        <Probe />
      </NotificationProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "Trigger" }).click();
    });
    expect(screen.getByText("Saved ok")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(4500);
    });

    expect(screen.queryByText("Saved ok")).not.toBeInTheDocument();
  });
});
