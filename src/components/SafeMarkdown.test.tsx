import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { SafeMarkdown } from "./SafeMarkdown";

describe("SafeMarkdown", () => {
  it("renders basic markdown elements", () => {
    render(
      <SafeMarkdown
        markdown={`# Title

Paragraph with **bold**.`}
      />,
    );

    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Title");
    expect(screen.getByText("bold")).toBeInTheDocument();
  });

  it("does not render raw html script nodes", () => {
    const { container } = render(
      <SafeMarkdown markdown={'<script>alert("xss")</script><img src=x onerror=alert(1)>'} />,
    );

    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("img")).toBeNull();
  });

  it("does not render clickable links", () => {
    const { container } = render(
      <SafeMarkdown markdown="[click](javascript:alert(1))" />,
    );

    expect(container.querySelector("a")).toBeNull();
  });
});
