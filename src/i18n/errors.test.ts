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
      "未检测到可用字幕。部分 AI 字幕可能需要在设置中配置 B 站 Cookie。",
    );
    expect(localizeError("en", "NO_SUBTITLE").title).toBe(
      "No usable subtitles found. Some AI subtitles may require a Bilibili cookie in Settings.",
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
