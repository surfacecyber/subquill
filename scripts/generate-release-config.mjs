#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  buildReleaseConfigPatch,
  serializeReleaseConfigPatch,
} from "./lib/release-config.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const defaultOutputPath = path.join(rootDir, ".release", "tauri.release.conf.json");

async function main() {
  const repository = process.env.GITHUB_REPOSITORY;
  const publicKey = process.env.TAURI_UPDATER_PUBLIC_KEY;
  const outputPath = process.env.RELEASE_CONFIG_OUTPUT ?? defaultOutputPath;

  const patch = buildReleaseConfigPatch({
    repository: repository ?? "",
    publicKey: publicKey ?? "",
  });

  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, serializeReleaseConfigPatch(patch), "utf8");

  process.stdout.write(
    `Wrote release config to ${path.relative(rootDir, outputPath)}\n`,
  );
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`generate-release-config: ${message}\n`);
  process.exit(1);
});
