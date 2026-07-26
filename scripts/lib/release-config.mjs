/** @typedef {{ bundle: { createUpdaterArtifacts: true }, plugins: { updater: { pubkey: string, endpoints: string[] } } }} ReleaseConfigPatch */

const GITHUB_REPO_PATTERN = /^[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+$/;

/**
 * @param {string} repository
 * @returns {string}
 */
export function parseGitHubRepository(repository) {
  const trimmed = repository.trim();
  if (!trimmed) {
    throw new Error("GITHUB_REPOSITORY is required (expected owner/repo).");
  }
  if (!GITHUB_REPO_PATTERN.test(trimmed)) {
    throw new Error(
      `Invalid GITHUB_REPOSITORY "${trimmed}". Expected owner/repo with alphanumeric segments.`,
    );
  }
  return trimmed;
}

/**
 * @param {string} repository
 * @returns {string}
 */
export function buildUpdaterEndpoint(repository) {
  const repo = parseGitHubRepository(repository);
  return `https://github.com/${repo}/releases/latest/download/latest.json`;
}

/**
 * @param {string} publicKey
 * @returns {string}
 */
export function normalizePublicKey(publicKey) {
  const trimmed = publicKey.trim();
  if (!trimmed) {
    throw new Error("TAURI_UPDATER_PUBLIC_KEY is required and must be non-empty.");
  }
  if (trimmed.includes("\n") || trimmed.includes("\r")) {
    throw new Error(
      "TAURI_UPDATER_PUBLIC_KEY must be a single-line value (no embedded newlines).",
    );
  }
  return trimmed;
}

/**
 * @param {{ repository: string, publicKey: string }} input
 * @returns {ReleaseConfigPatch}
 */
export function buildReleaseConfigPatch({ repository, publicKey }) {
  const repo = parseGitHubRepository(repository);
  const pubkey = normalizePublicKey(publicKey);

  return {
    bundle: {
      createUpdaterArtifacts: true,
    },
    plugins: {
      updater: {
        pubkey,
        endpoints: [buildUpdaterEndpoint(repo)],
      },
    },
  };
}

/**
 * @param {ReleaseConfigPatch} patch
 * @returns {string}
 */
export function serializeReleaseConfigPatch(patch) {
  return `${JSON.stringify(patch, null, 2)}\n`;
}
