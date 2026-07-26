#!/usr/bin/env node

import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  assertVersionsConsistent,
  parseTagVersion,
  readProjectVersions,
} from "./lib/check-versions.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");

function readTagArg(argv) {
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--tag") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error("--tag requires a value (for example v1.2.3).");
      }
      return value;
    }
    if (arg.startsWith("--tag=")) {
      return arg.slice("--tag=".length);
    }
  }

  const envTag = process.env.GITHUB_REF_NAME ?? process.env.RELEASE_TAG;
  return envTag ?? undefined;
}

async function main() {
  const versions = await readProjectVersions(rootDir);
  const tag = readTagArg(process.argv.slice(2));
  const tagVersion = tag ? parseTagVersion(tag) : undefined;
  const projectVersion = assertVersionsConsistent({ versions, tagVersion });

  process.stdout.write(`Version check passed: ${projectVersion}\n`);
  if (tagVersion) {
    process.stdout.write(`Release tag matches project version: v${tagVersion}\n`);
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`check-versions: ${message}\n`);
  process.exit(1);
});
