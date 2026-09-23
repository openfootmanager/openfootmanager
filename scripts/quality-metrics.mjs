#!/usr/bin/env node
/**
 * The ratchet: numbers that are allowed to fall and not to rise.
 *
 * This exists because some debt cannot honestly be cleared in the pull request that measures it.
 * Enabling a lint rule whose findings nobody has fixed produces a reporter, and this project has
 * watched two of those sit ignored for a year. The alternative to a reporter is not "fix 150
 * components today" — it is to write the number down and refuse to let it grow, so the debt can
 * be paid a screen at a time while nothing adds to it.
 *
 *   node scripts/quality-metrics.mjs --check   compare against the baseline, exit 1 on any rise
 *   node scripts/quality-metrics.mjs --write   regenerate the baseline
 *
 * A metric going *down* is always allowed and should be committed with the change that earned
 * it — `--check` says so when it notices.
 */

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const repoRoot = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const baselinePath = join(repoRoot, "quality-baseline.json");

const RUST_MAX_LINES = 1000;
const TS_MAX_LINES = 500;

const SKIP_DIRS = new Set(["node_modules", "target", "dist", ".git", "gen", ".claude", "coverage"]);

function walk(dir, predicate, found = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      walk(join(dir, entry.name), predicate, found);
    } else if (predicate(entry.name)) {
      found.push(join(dir, entry.name));
    }
  }
  return found;
}

const rel = (p) => relative(repoRoot, p).split(sep).join("/");
const lineCount = (p) => readFileSync(p, "utf8").split("\n").length;

/** Files over their language's threshold, with the line count that put them there. */
function oversized() {
  const rust = walk(join(repoRoot, "src-tauri"), (n) => n.endsWith(".rs"));
  const ts = walk(join(repoRoot, "src"), (n) => n.endsWith(".ts") || n.endsWith(".tsx"));

  const allowed = {};
  const aggregate = { rust: 0, ts: 0 };

  for (const [files, limit, key] of [
    [rust, RUST_MAX_LINES, "rust"],
    [ts, TS_MAX_LINES, "ts"],
  ]) {
    for (const file of files) {
      const lines = lineCount(file);
      if (lines <= limit) continue;
      allowed[rel(file)] = lines;
      aggregate[key] += lines;
    }
  }

  return { aggregate, allowed };
}

/**
 * Strip comments before counting.
 *
 * Without this the metrics count their own prose: the first baseline recorded five `: any`
 * annotations that were all the word "any" in an English sentence, including one inside a
 * comment explaining why an `any` had been removed. A number that rises when somebody writes a
 * clear comment is worse than no number.
 *
 * Deliberately naive — it does not understand a `//` inside a string literal. For counting
 * suppressions and known anti-patterns that is the right trade: the alternative is a parser, and
 * these are directional signals rather than numbers to defend in review.
 */
function stripComments(source) {
  return source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|\s)\/\/.*$/gm, "$1");
}

/**
 * How many times a pattern appears across a set of files.
 *
 * Two variants, and choosing the wrong one is how the first two baselines were wrong in both
 * directions. `countInCode` ignores comments, for things that are code — an `any`, a clone. Use
 * `countInSource` for anything that *lives* in a comment: a `biome-ignore`, an `eslint-disable`.
 * Stripping comments before counting those reported zero suppressions in a tree that had seven.
 */
function countIn(files, pattern, transform) {
  let total = 0;
  for (const file of files) {
    const matches = transform(readFileSync(file, "utf8")).match(pattern);
    if (matches) total += matches.length;
  }
  return total;
}

const countInCode = (files, pattern) => countIn(files, pattern, stripComments);
const countInSource = (files, pattern) => countIn(files, pattern, (s) => s);

function suppressions() {
  // `scripts/` and `.github/scripts/` are included deliberately: three of the suppressions this
  // repository carries live there, and counting only `src/` would have left them outside the
  // ratchet entirely — a place to put a `biome-ignore` where nothing notices.
  const ts = [
    ...walk(join(repoRoot, "src"), (n) => n.endsWith(".ts") || n.endsWith(".tsx")),
    ...walk(join(repoRoot, "scripts"), (n) => n.endsWith(".mjs") || n.endsWith(".ts")),
    ...walk(join(repoRoot, ".github", "scripts"), (n) => n.endsWith(".mjs")),
  ];
  const rust = walk(join(repoRoot, "src-tauri"), (n) => n.endsWith(".rs"));

  return {
    // `: any` and `as any` are counted separately from each other because they fail differently:
    // one is a declaration nobody tightened, the other is an assertion somebody made.
    tsAnyAnnotation: countInCode(ts, /:\s*any\b/g),
    tsAnyAssertion: countInCode(ts, /\bas\s+any\b/g),
    // Anchored to the directive form — a comment that *starts* with the marker — rather than the
    // bare word. Counting every occurrence meant this file's own explanation of what a
    // `biome-ignore` is counted as three suppressions, so writing documentation raised the
    // ratchet and could fail CI without anyone adding a directive. That is the same mistake as
    // counting the word "any" in prose, in the opposite direction.
    tsIgnore: countInSource(ts, /^\s*(\/\/|\*|\/\*)\s*@ts-(ignore|expect-error)\b/gm),
    biomeIgnore: countInSource(ts, /^\s*(\/\/|\*|\/\*)\s*biome-ignore\b/gm),
    // Decorative: this repository has never had ESLint, so any of these suppress nothing at all.
    // The number should only ever go down.
    eslintDisable: countInSource(ts, /^\s*(\/\/|\*|\/\*)\s*eslint-disable/gm),
    rustAllow: countInCode(rust, /#\[allow\(/g),
    rustExpect: countInCode(rust, /#\[expect\(/g),
    rustUnsafeBlocks: countInCode(rust, /\bunsafe\s*\{/g),
  };
}

/**
 * Lint findings that are knowingly outstanding.
 *
 * Read from Biome's own JSON so the number cannot drift from what the linter actually says, and
 * so a rule that gets *enabled* later simply drops out of here.
 */
function lintDebt() {
  let raw;
  try {
    raw = execFileSync("npm", ["exec", "--no", "--", "biome", "lint", "--reporter=json"], {
      cwd: repoRoot,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch (error) {
    // biome exits non-zero whenever it has findings, which is the normal case here.
    raw = error.stdout;
  }
  if (!raw) throw new Error("biome produced no JSON — is it installed? (npm ci)");

  const counts = {};
  for (const d of JSON.parse(raw).diagnostics ?? []) {
    const rule = d.category?.replace(/^lint\//, "");
    if (!rule) continue;
    counts[rule] = (counts[rule] ?? 0) + 1;
  }
  return Object.fromEntries(Object.entries(counts).sort(([a], [b]) => a.localeCompare(b)));
}

/**
 * Dead code, counted rather than deleted.
 *
 * Knip's raw number here was 138 and almost all of it was misconfiguration: without entry points
 * it could not see that `scripts/perf/*.mjs` are run by hand, so it called them unused. Properly
 * configured it reports 35, and most of *those* are deliberate — barrel re-exports from
 * `components/ui/index.ts` and `teamProfile/index.ts` are the public surface this project asks
 * you to search before writing a helper, and the types on `store/gameStore.ts` document the wire
 * shape the Rust side sends whether or not TypeScript happens to reference them today.
 *
 * So this is a floor, not a hit list: a genuinely dead export can be removed and the number
 * falls, but nothing new can accumulate behind it.
 */
function deadCode() {
  let raw;
  try {
    raw = execFileSync(
      "npm",
      ["exec", "--no", "--", "knip", "--reporter", "json", "--no-progress"],
      {
        cwd: repoRoot,
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
        stdio: ["ignore", "pipe", "ignore"],
      },
    );
  } catch (error) {
    raw = error.stdout;
  }
  if (!raw) throw new Error("knip produced no JSON — is it installed? (npm ci)");

  const counts = { unusedFiles: 0, unusedExports: 0, unusedTypes: 0, duplicateExports: 0 };
  for (const issue of JSON.parse(raw).issues ?? []) {
    if (issue.files?.length) counts.unusedFiles += issue.files.length;
    counts.unusedExports += issue.exports?.length ?? 0;
    counts.unusedTypes += issue.types?.length ?? 0;
    counts.duplicateExports += issue.duplicates?.length ?? 0;
  }
  return counts;
}

/**
 * Named anti-patterns: things already found by hand that a general tool will not catch.
 *
 * Each is a directional signal rather than a number to defend in review — a rename defeats the
 * regex. They are here so a 53rd copy of a known mistake has to argue for itself.
 */
function antiPatterns() {
  const rustSrc = walk(join(repoRoot, "src-tauri", "src"), (n) => n.endsWith(".rs"));
  const ts = walk(join(repoRoot, "src"), (n) => n.endsWith(".ts") || n.endsWith(".tsx"));

  return {
    // Cloning the whole Game to read one field: ~440 teams and ~9.7k players per call. This
    // counts production and test sites together; a clone inside `#[cfg(test)]` is not the
    // performance problem, but telling them apart needs a parser and the floor matters more than
    // the split.
    fullGameClones: countInCode(
      rustSrc,
      /get_game\(\s*\|[a-z_]+(:\s*&Game)?\|\s*[a-z_]+\.clone\(\)\s*\)/g,
    ),
    // A feature folder that has become a shared library: imports reaching into components/squad
    // from outside it.
    // Relative, because that is how this codebase spells them: `../squad/SquadTab.helpers`, not
    // `components/squad/...`. The first version of this matched the absolute form and reported 1
    // where the real number is in the dozens.
    crossFeatureSquadImports: ts
      .filter((f) => !rel(f).startsWith("src/components/squad/"))
      .reduce(
        (n, f) =>
          n +
          (stripComments(readFileSync(f, "utf8")).match(/from\s+"[^"]*\/squad\//g)?.length ?? 0),
        0,
      ),
  };
}

function collect() {
  const { aggregate, allowed } = oversized();
  return {
    oversizedAggregate: aggregate,
    allowedOversized: Object.fromEntries(
      Object.entries(allowed).sort(([a], [b]) => a.localeCompare(b)),
    ),
    suppressions: suppressions(),
    lintDebt: lintDebt(),
    deadCode: deadCode(),
    antiPatterns: antiPatterns(),
  };
}

function compare(current, baseline) {
  const failures = [];
  const improvements = [];

  const check = (label, now, then) => {
    if (typeof then !== "number") {
      failures.push(`${label}: ${now} is not in the baseline. Run \`npm run quality:baseline\`.`);
    } else if (now > then) {
      failures.push(`${label}: ${then} → ${now} (+${now - then})`);
    } else if (now < then) {
      improvements.push(`${label}: ${then} → ${now}`);
    }
  };

  for (const group of [
    "oversizedAggregate",
    "suppressions",
    "antiPatterns",
    "lintDebt",
    "deadCode",
  ]) {
    for (const [key, value] of Object.entries(current[group])) {
      check(`${group}.${key}`, value, baseline[group]?.[key]);
    }
    // A metric that vanished entirely is an improvement, not a missing baseline.
    for (const key of Object.keys(baseline[group] ?? {})) {
      if (!(key in current[group])) improvements.push(`${group}.${key}: gone`);
    }
  }

  // A file over threshold with no baseline entry fails. Adding the entry is allowed — in the
  // same pull request, where a reviewer sees the argument for it.
  for (const [file, lines] of Object.entries(current.allowedOversized)) {
    const was = baseline.allowedOversized?.[file];
    if (was === undefined) {
      failures.push(`allowedOversized: ${file} is ${lines} lines and has no baseline entry.`);
    } else if (lines > was) {
      failures.push(`allowedOversized: ${file} ${was} → ${lines} (+${lines - was})`);
    } else if (lines < was) {
      improvements.push(`allowedOversized: ${file} ${was} → ${lines}`);
    }
  }

  // A stale entry fails too. Without this the map only ever grows, and a dead entry is a
  // standing permission slip: delete a 2000-line file today, and a year later a new file at that
  // exact path can land at 2000 lines silently.
  for (const file of Object.keys(baseline.allowedOversized ?? {})) {
    if (!(file in current.allowedOversized)) {
      improvements.push(`allowedOversized: ${file} is no longer oversized — remove its entry`);
      failures.push(`allowedOversized: ${file} has a stale entry. Delete the line.`);
    }
  }

  return { failures, improvements };
}

const mode = process.argv[2];
const current = collect();

if (mode === "--write") {
  writeFileSync(baselinePath, `${JSON.stringify(current, null, 2)}\n`);
  console.log(`quality-metrics: baseline written to ${rel(baselinePath)}`);
  process.exit(0);
}

if (mode !== "--check") {
  console.error("usage: quality-metrics.mjs --check | --write");
  process.exit(2);
}

let baseline;
try {
  baseline = JSON.parse(readFileSync(baselinePath, "utf8"));
} catch {
  console.error(
    `quality-metrics: cannot read ${rel(baselinePath)}. Run \`npm run quality:baseline\`.`,
  );
  process.exit(1);
}

const { failures, improvements } = compare(current, baseline);

for (const line of improvements) console.log(`  improved  ${line}`);

if (failures.length > 0) {
  console.error("\nquality-metrics: these numbers may not rise.\n");
  for (const line of failures) console.error(`  ${line}`);
  console.error(
    "\nIf the rise is deliberate, edit quality-baseline.json in this same pull request so the" +
      "\nargument for it is visible in review. `npm run quality:baseline` regenerates the file.",
  );
  process.exit(1);
}

console.log(
  improvements.length > 0
    ? "\nquality-metrics: nothing rose, and some things fell. Commit the regenerated baseline."
    : "quality-metrics: nothing rose",
);
