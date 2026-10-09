#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# tauri-action reuses published tags too. Stable checks derive the Tauri version;
# nightly checks receive RELEASE_TAG from prepare-nightly. Recheck before uploads.
node --input-type=module <<'NODE'
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

function refuse(message) {
  console.error(message);
  process.exit(1);
}

const repository = process.env.GITHUB_REPOSITORY;
if (!repository || !/^[\w.-]+\/[\w.-]+$/.test(repository)) {
  refuse("Invalid repository or Tauri release version");
}
let tag = process.env.RELEASE_TAG;
if (tag === undefined) {
  const version = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
  if (typeof version !== "string" || !/^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/.test(version)) {
    refuse("Invalid repository or Tauri release version");
  }
  tag = `v${version}`;
}
// The action rewrites aliases and trims inputs; lookup, upload and finalization must agree.
if (tag !== tag.trim() || tag.includes("refs/tags/") || tag.includes("__VERSION__")) {
  refuse("Use an exact release tag without tauri-action aliases or surrounding whitespace");
}
const reference = spawnSync("git", ["check-ref-format", `refs/tags/${tag}`], { encoding: "utf8" });
if (reference.error) refuse("Unable to validate release tag");
if (reference.status !== 0) refuse("Invalid release tag");
const response = spawnSync("gh", [
  "api", "--include", "--method", "GET",
  `repos/${repository}/releases/tags/${encodeURIComponent(tag)}`,
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
