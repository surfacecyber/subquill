import { describe, expect, it } from "vitest";

import { resolveUiLocale, t } from "../i18n";

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
});
