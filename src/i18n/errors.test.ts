import { describe, expect, it } from "vitest";

import { categorizeErrorCode, localizeError } from "./errors";

describe("error localization", () => {
  it("categorizes storage errors without network", () => {
    expect(categorizeErrorCode("STORAGE_ERROR")).toBe("storage");
    expect(categorizeErrorCode("SERIALIZATION_ERROR")).toBe("storage");
    expect(categorizeErrorCode("NETWORK_ERROR")).not.toBe("storage");
  });

  it("categorizes network errors separately", () => {
    expect(categorizeErrorCode("NETWORK_ERROR")).toBe("network");
  });

  it("localizes NO_SUBTITLE specifically", () => {
    expect(localizeError("zh", "NO_SUBTITLE").title).toBe(
      "该视频暂无可用的在线字幕（含 AI 字幕）。通常与 Cookie 无关；可换有字幕的视频，或稍后再试。",
    );
    expect(localizeError("en", "NO_SUBTITLE").title).toBe(
      "No online subtitle track is available for this video (including AI captions). This usually isn't a cookie issue; try another video or retry later.",
    );
  });

  it("localizes specific llm auth errors", () => {
    const localized = localizeError("en", "LLM_AUTH_ERROR", "raw secret detail");
    expect(localized.title).toContain("authentication");
    expect(localized.detail).toBeUndefined();
  });

  it("localizes job timeout specifically", () => {
    expect(localizeError("en", "JOB_TIMEOUT").title).toContain("timed out");
  });

  it("includes detail for unknown errors only", () => {
    const localized = localizeError("en", "UNKNOWN_ERROR", "technical detail");
    expect(localized.detail).toBe("technical detail");
  });
});
