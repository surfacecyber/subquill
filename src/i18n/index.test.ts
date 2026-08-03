import { describe, expect, it } from "vitest";

import { progressPercent, resolveUiLocale, t } from "../i18n";

describe("i18n", () => {
  it("resolves zh when locale is zh", () => {
    expect(resolveUiLocale("zh")).toBe("zh");
  });

  it("falls back to en for non-zh system locale", () => {
    expect(resolveUiLocale("system", "en-US")).toBe("en");
  });

  it("falls back to zh for zh system locale", () => {
    expect(resolveUiLocale("system", "zh-CN")).toBe("zh");
  });

  it("returns localized strings", () => {
    expect(t("en", "setupTitle")).toBe("Initial setup");
    expect(t("zh", "setupTitle")).toBe("首次设置");
  });

  it("maps stage and chunk progress to a percent", () => {
    expect(progressPercent("fetching_subtitles")).toBe(0);
    expect(progressPercent("chunking")).toBe(15);
    expect(progressPercent("calling_llm", 0, 4)).toBe(25);
    expect(progressPercent("calling_llm", 2, 4)).toBe(55);
    expect(progressPercent("merging")).toBe(85);
    expect(progressPercent("done")).toBe(100);
  });
});
