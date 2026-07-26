import { describe, expect, it } from "vitest";

import {
  buildReleaseConfigPatch,
  buildUpdaterEndpoint,
  normalizePublicKey,
  parseGitHubRepository,
  serializeReleaseConfigPatch,
} from "../lib/release-config.mjs";

describe("release-config", () => {
  it("parses a valid GitHub repository slug", () => {
    expect(parseGitHubRepository("acme-corp/open-note")).toBe("acme-corp/open-note");
  });

  it("rejects invalid repository slugs", () => {
    expect(() => parseGitHubRepository("")).toThrow(/GITHUB_REPOSITORY/);
    expect(() => parseGitHubRepository("missing-owner")).toThrow(/Invalid GITHUB_REPOSITORY/);
  });

  it("builds the GitHub latest.json endpoint from the repository slug", () => {
    expect(buildUpdaterEndpoint("owner/repo")).toBe(
      "https://github.com/owner/repo/releases/latest/download/latest.json",
    );
  });

  it("rejects multiline public keys", () => {
    expect(() => normalizePublicKey("line1\nline2")).toThrow(/single-line/);
  });

  it("builds a release config patch with escaped JSON values", () => {
    const patch = buildReleaseConfigPatch({
      repository: "org/app",
      publicKey: "dW5zYWZl\"key",
    });

    expect(patch.bundle.createUpdaterArtifacts).toBe(true);
    expect(patch.plugins.updater.endpoints).toEqual([
      "https://github.com/org/app/releases/latest/download/latest.json",
    ]);
    expect(patch.plugins.updater.pubkey).toBe('dW5zYWZl"key');

    const serialized = serializeReleaseConfigPatch(patch);
    expect(serialized).toContain('"createUpdaterArtifacts": true');
    expect(() => JSON.parse(serialized)).not.toThrow();
  });
});
