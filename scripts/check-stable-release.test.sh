#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
node --input-type=module <<'NODE'
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(
  ".github/workflows/tauri-action.yml",
  "utf8",
);

// Given platform uploads precede the AppImage check, when a build uploads, then it stays private.
test("stable platform uploads stay drafts until AppImage validation finishes", () => {
  assert.match(workflow, /releaseDraft: true/);
  assert.doesNotMatch(workflow, /releaseDraft: false/);
});

// Given any matrix build or AppImage check fails, when jobs finish, then publication is skipped.
test("stable publication requires the whole platform matrix to succeed", () => {
  const finalize = workflow.split("\n  finalize-stable:")[1];
  assert.ok(finalize, "a finalization job must exist");
  assert.match(finalize, /needs: publish-tauri/);
  assert.match(finalize, /needs\.publish-tauri\.result == 'success'/);
  assert.doesNotMatch(finalize, /always\(\)/);
});

// Given all platforms pass, when finalization runs, then only the versioned stable draft is published.
test("stable finalization publishes the package version without a prerelease flag", () => {
  const finalize = workflow.split("\n  finalize-stable:")[1];
  assert.ok(finalize, "a finalization job must exist");
  assert.match(finalize, /require\('\.\/package\.json'\)\.version/);
  assert.match(finalize, /gh release edit "v\$release_version" --draft=false --prerelease=false/);
});
NODE
