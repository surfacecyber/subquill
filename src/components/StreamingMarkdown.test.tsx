import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { splitStreamingMarkdown, StreamingMarkdown } from "./StreamingMarkdown";

describe("StreamingMarkdown", () => {
  it("splits on the last complete blank line", () => {
    expect(splitStreamingMarkdown("## Summary\n\nHello\n\nworld")).toEqual({
      committed: "## Summary\n\nHello",
      pending: "world",
    });
  });

  it("shows thinking copy when there is no draft yet", () => {
    render(
      <StreamingMarkdown markdown="" thinking thinkingLabel="Model is thinking…" />,
    );
    expect(screen.getByRole("status")).toHaveTextContent("Model is thinking…");
  });

  it("renders committed markdown as a heading", () => {
    render(
      <StreamingMarkdown
        markdown={"## Summary\n\nHello"}
        thinkingLabel="thinking"
      />,
    );
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "Summary",
    );
    expect(screen.getByText("Hello")).toBeInTheDocument();
  });
});
