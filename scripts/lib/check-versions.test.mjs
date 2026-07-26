import { describe, expect, it } from "vitest";

import {
  assertSemver,
  assertVersionsConsistent,
  parseTagVersion,
  readCargoVersion,
} from "../lib/check-versions.mjs";

describe("check-versions", () => {
  it("parses release tags", () => {
    expect(parseTagVersion("v1.2.3")).toBe("1.2.3");
    expect(() => parseTagVersion("1.2.3")).toThrow(/Invalid release tag/);
  });

  it("reads Cargo.toml versions", () => {
    const cargo = `[package]\nname = "opennote"\nversion = "0.1.0"\n`;
    expect(readCargoVersion(cargo)).toBe("0.1.0");
  });

  it("validates semver strings", () => {
    expect(() => assertSemver("0.1.0", "package.json")).not.toThrow();
    expect(() => assertSemver("v0.1.0", "package.json")).toThrow(/not valid semver/);
  });

  it("requires all project versions to match", () => {
    expect(() =>
      assertVersionsConsistent({
        versions: {
          packageJson: "0.1.0",
          cargoToml: "0.1.1",
          tauriConf: "0.1.0",
        },
      }),
    ).toThrow(/Version mismatch/);

    expect(
      assertVersionsConsistent({
        versions: {
          packageJson: "0.1.0",
          cargoToml: "0.1.0",
          tauriConf: "0.1.0",
        },
        tagVersion: "0.1.0",
      }),
    ).toBe("0.1.0");
  });
});
