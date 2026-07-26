import { readFile } from "node:fs/promises";
import path from "node:path";

const SEMVER_PATTERN = /^\d+\.\d+\.\d+$/;
const TAG_PATTERN = /^v(\d+\.\d+\.\d+)$/;

/**
 * @param {string} tag
 * @returns {string}
 */
export function parseTagVersion(tag) {
  const trimmed = tag.trim();
  const match = TAG_PATTERN.exec(trimmed);
  if (!match) {
    throw new Error(`Invalid release tag "${tag}". Expected format vX.Y.Z.`);
  }
  return match[1];
}

/**
 * @param {string} version
 * @param {string} label
 */
export function assertSemver(version, label) {
  if (!SEMVER_PATTERN.test(version)) {
    throw new Error(`${label} version "${version}" is not valid semver (X.Y.Z).`);
  }
}

/**
 * @param {string} cargoToml
 * @returns {string}
 */
export function readCargoVersion(cargoToml) {
  const match = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error("Could not read version from src-tauri/Cargo.toml.");
  }
  return match[1];
}

/**
 * @param {string} rootDir
 * @returns {Promise<{ packageJson: string, cargoToml: string, tauriConf: string }>}
 */
export async function readProjectVersions(rootDir) {
  const [packageJsonRaw, cargoTomlRaw, tauriConfRaw] = await Promise.all([
    readFile(path.join(rootDir, "package.json"), "utf8"),
    readFile(path.join(rootDir, "src-tauri", "Cargo.toml"), "utf8"),
    readFile(path.join(rootDir, "src-tauri", "tauri.conf.json"), "utf8"),
  ]);

  const packageJson = JSON.parse(packageJsonRaw);
  const tauriConf = JSON.parse(tauriConfRaw);
  const cargoToml = readCargoVersion(cargoTomlRaw);

  const versions = {
    packageJson: packageJson.version,
    cargoToml,
    tauriConf: tauriConf.version,
  };

  for (const [label, version] of Object.entries(versions)) {
    assertSemver(version, label);
  }

  return versions;
}

/**
 * @param {{ versions: Record<string, string>, tagVersion?: string }} input
 */
export function assertVersionsConsistent({ versions, tagVersion }) {
  const unique = new Set(Object.values(versions));
  if (unique.size !== 1) {
    const details = Object.entries(versions)
      .map(([file, version]) => `${file}=${version}`)
      .join(", ");
    throw new Error(`Version mismatch across project files: ${details}`);
  }

  const projectVersion = [...unique][0];
  if (tagVersion && projectVersion !== tagVersion) {
    throw new Error(
      `Release tag version v${tagVersion} does not match project version ${projectVersion}.`,
    );
  }

  return projectVersion;
}
