#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# tauri-action reuses published tags too. Check before any build or asset upload so a
# same-version rerun cannot replace a validated public AppImage with an unchecked one.
node --input-type=module <<'NODE'
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

function refuse(message) {
  console.error(message);
  process.exit(1);
}

const repository = process.env.GITHUB_REPOSITORY;
const version = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
if (!repository || !/^[\w.-]+\/[\w.-]+$/.test(repository) ||
    typeof version !== "string" || !/^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/.test(version)) {
  refuse("Invalid repository or Tauri release version");
}
const tag = `v${version}`;
const response = spawnSync("gh", [
  "api", "--include", "--method", "GET",
  `repos/${repository}/releases/tags/${tag}`,
], { encoding: "utf8" });
const httpStatus = Number(response.stdout?.match(/^HTTP\/\S+\s+(\d+)/)?.[1]);
if (httpStatus === 404 && response.status !== 0 && !response.error) {
  console.log(`${tag} does not exist; a new draft release may be built`);
} else {
  if (response.error || response.status !== 0 || httpStatus !== 200) {
    refuse(`Unable to check ${tag}; refusing to build or upload`);
  }
  let release;
  try {
    release = JSON.parse(response.stdout.split(/\r?\n\r?\n/).slice(1).join("\n\n"));
  } catch {
    refuse(`Invalid release response for ${tag}`);
  }
  if (typeof release?.draft !== "boolean") {
    refuse(`Invalid release response for ${tag}`);
  }
  if (!release.draft) {
    refuse(`${tag} is already published; refusing to replace its assets`);
  }
  console.log(`${tag} is a draft; publication may resume`);
}
NODE
