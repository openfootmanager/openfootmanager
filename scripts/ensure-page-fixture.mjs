#!/usr/bin/env node
// Generates the page harness's career before `npm test`, `npm run test:watch`, `npm run build`,
// `npm run knip` (so `npm run preflight`) and `npm run dev` (so `tauri dev` too) when it is
// missing. Knip reports the harness's import of it as unresolved without it.
//
// src/dev/pageHarness/fixture.generated.ts is written by `npm run fixture:pages` from the Rust
// side (src-tauri/crates/ofm_core/examples/page_fixture.rs) and is not committed: committed, most
// PRs that touched a page or the world regenerated it, and two of them either conflicted or merged
// into a copy no fresh generation produced, which dropped them from the merge queue. The frontend
// imports it (the harness, and tests that read its career), so a fresh checkout needs it before
// the first test or build. CI's frontend job generates it itself, so there it is always present
// and current; locally it is generated once and then left alone. Run `npm run fixture:pages`
// again after changing what it is generated from.

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fixture = path.join(root, "src/dev/pageHarness/fixture.generated.ts");

if (existsSync(fixture)) {
  process.exit(0);
}

console.log(
  "ensure-page-fixture: the page harness's career is missing; generating it (npm run fixture:pages).",
);
// Node refuses to spawn a .cmd file on Windows without a shell.
const windows = process.platform === "win32";
const result = spawnSync(windows ? "npm.cmd" : "npm", ["run", "fixture:pages"], {
  cwd: root,
  stdio: "inherit",
  shell: windows,
});

if (result.error || result.status !== 0) {
  console.error(
    "ensure-page-fixture: could not generate src/dev/pageHarness/fixture.generated.ts. " +
      "It is built by the Rust side, so this needs the Rust toolchain (rust-toolchain.toml). " +
      "Run `npm run fixture:pages` to see the error.",
  );
  process.exit(result.status ?? 1);
}
