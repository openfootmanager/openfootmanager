#!/usr/bin/env node
/**
 * Render collected benchmark results as a markdown table for docs/LINUX_GRAPHICS.md.
 *
 *   node scripts/perf/report.mjs [--in DIR]
 *
 * Rows are printed in ladder order — hardware-accelerated configurations first — so the table
 * reads as "how much does each workaround cost us".
 */

import { readdir, readFile } from "node:fs/promises";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const readFlag = (name, fallback) => {
  const index = args.indexOf(name);
  return index === -1 ? fallback : args[index + 1];
};

const inDir = resolve(readFlag("--in", "scripts/perf/results"));

// Same order as run-matrix.sh, with the acceleration boundary marked.
const LADDER = [
  "baseline",
  "auto",
  "explicit-sync",
  "render-intel",
  "render-nvidia",
  "disable-gbm",
  "force-shm",
  "shipped",
  "no-compositing",
  "xwayland",
  "chromium",
  "firefox",
];

const files = (await readdir(inDir)).filter((name) => name.endsWith(".json"));
if (files.length === 0) {
  console.error(`No results in ${inDir}. Run scripts/perf/run-matrix.sh first.`);
  process.exit(1);
}

const results = await Promise.all(
  files.map(async (name) => JSON.parse(await readFile(resolve(inDir, name), "utf8"))),
);

results.sort((a, b) => {
  const rank = (label) => {
    const index = LADDER.indexOf(label);
    return index === -1 ? LADDER.length : index;
  };
  return rank(a.label) - rank(b.label) || a.label.localeCompare(b.label);
});

const phaseNames = [...new Set(results.flatMap((r) => r.phases.map((p) => p.name)))];

const header = ["configuration", ...phaseNames.flatMap((n) => [`${n} p50`, `${n} p95`, `${n} drop`])];
const divider = header.map(() => "---");

const rows = results.map((result) => {
  const cells = [`\`${result.label}\``];
  for (const name of phaseNames) {
    const phase = result.phases.find((p) => p.name === name);
    cells.push(phase ? `${phase.p50}` : "—", phase ? `${phase.p95}` : "—", phase ? `${phase.dropped}` : "—");
  }
  return cells;
});

const toRow = (cells) => `| ${cells.join(" | ")} |`;

console.log(`Frame-to-frame delta in ms (lower is better); \`drop\` counts frames over 1.5x budget.`);
console.log();
console.log(toRow(header));
console.log(toRow(divider));
for (const row of rows) console.log(toRow(row));
console.log();

for (const result of results) {
  const budget = result.frameBudgetMs;
  console.log(
    `- \`${result.label}\`: idle frame budget ${budget} ms (~${Math.round(1000 / budget)} Hz), ` +
      `viewport ${result.viewport.width}x${result.viewport.height}, dpr ${result.devicePixelRatio}` +
      (result.longAnimationFrames >= 0 ? `, ${result.longAnimationFrames} long animation frames` : ""),
  );
}
