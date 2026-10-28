#!/usr/bin/env node
/**
 * Derives the scoreline calibration bands in `sim-bench` from real results.
 *
 * # Why this script exists
 *
 * The bands in `crates/sim-bench/src/targets.rs` were, until now, twenty-one
 * numbers with no recorded origin. Five of them describe nothing but the
 * scoreline — goals, clean sheets, both-teams-scored, home wins — and a
 * scoreline is the one football statistic that is available under a licence we
 * can actually use. So those five stop being recollection and become a
 * measurement, regenerable from a pinned upstream revision.
 *
 * # Source
 *
 * openfootball/football.json — CC0-1.0, public domain, "use as you please with
 * no restrictions whatsoever". Results only, no per-match statistics, which is
 * exactly the subset this script needs.
 *
 * # The band rule, fixed before the first number was computed
 *
 * Each league-season contributes ONE value per metric. The band is the
 * **10th to 90th percentile across those league-seasons**, stored at full
 * precision. The report rounds only when it prints: rounding the stored band
 * would move a value at its edge from one side to the other.
 *
 * So a band says: *a value outside this is outside the range where eight in ten
 * real top-flight league-seasons fell.* It is deliberately not a confidence
 * interval on the pooled mean — that would be ±0.03 on goals/game and would
 * assert the engine must reproduce one particular league, which is not the
 * claim. The engine invents its own leagues; it should land where real ones do.
 *
 * This rule is written down here, and was chosen before any output was seen,
 * because the tempting alternative is to pick the interval that the engine
 * happens to sit inside. A band fitted to the engine is not a target.
 *
 * # Determinism
 *
 * Running this twice against the same pinned commit must produce a
 * byte-identical artifact, so it records the upstream commit and its date
 * rather than the wall clock at the time it ran. Every file is read from that
 * commit with `git show`, never from the clone's working tree, so a checkout of
 * another revision, or local edits, cannot change the numbers.
 *
 * # Usage
 *
 *   node scripts/calibrate-scorelines.mjs --repo <path-to-football.json-clone>
 *
 * Clone with (a blobless clone fetches only the forty files it reads):
 *   git clone --filter=blob:none --no-checkout https://github.com/openfootball/football.json.git
 */

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

/** The upstream revision these bands were derived from. */
const PINNED_COMMIT = "4e4146c901b62bcafa1b6deabb7e4a3fccdc9b1f";

/**
 * Big-five top flights. The game simulates a generic modern top division, so
 * the reference population is the top divisions, not the pyramid.
 */
const LEAGUES = {
  "en.1": "English Premier League",
  "es.1": "Spanish La Liga",
  "de.1": "German Bundesliga",
  "it.1": "Italian Serie A",
  "fr.1": "French Ligue 1",
};

/**
 * Every season in which all five leagues ran to completion, except two.
 *
 * 2019-20 and 2020-21 are excluded: both were played wholly or largely behind
 * closed doors, and Ligue 1 2019-20 was abandoned outright at 279 of 380
 * matches. Empty stadiums moved home advantage more than anything else in the
 * data, and home advantage is one of the five metrics being derived — so
 * including them would bake a pandemic into the engine's idea of a normal
 * season.
 */
const SEASONS = [
  "2014-15",
  "2015-16",
  "2016-17",
  "2017-18",
  "2018-19",
  "2021-22",
  "2022-23",
  "2023-24",
];

/** Each metric: how to read one match, and how to render the band. */
const METRICS = [
  { key: "goals_per_game", label: "Goals/game", unit: "per_game", of: (h, a) => h + a },
  {
    key: "clean_sheets_home_pct",
    label: "Clean sheets (home)",
    unit: "percent",
    of: (_h, a) => (a === 0 ? 100 : 0),
  },
  {
    key: "clean_sheets_away_pct",
    label: "Clean sheets (away)",
    unit: "percent",
    of: (h) => (h === 0 ? 100 : 0),
  },
  {
    key: "both_teams_scored_pct",
    label: "Both teams scored",
    unit: "percent",
    of: (h, a) => (h > 0 && a > 0 ? 100 : 0),
  },
  { key: "home_win_pct", label: "Home win %", unit: "percent", of: (h, a) => (h > a ? 100 : 0) },
];

/** Linear-interpolated percentile, the definition R and NumPy default to. */
function percentile(sorted, p) {
  if (sorted.length === 1) return sorted[0];
  const rank = (p / 100) * (sorted.length - 1);
  const lo = Math.floor(rank);
  const hi = Math.ceil(rank);
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (rank - lo);
}

function main() {
  const repoFlag = process.argv.indexOf("--repo");
  if (repoFlag === -1 || !process.argv[repoFlag + 1]) {
    console.error("usage: calibrate-scorelines.mjs --repo <path-to-football.json>");
    process.exit(2);
  }
  const repo = process.argv[repoFlag + 1];

  const atPin = (relative) =>
    execFileSync("git", ["-C", repo, "show", `${PINNED_COMMIT}:${relative}`], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    });
  const commitDate = execFileSync(
    "git",
    ["-C", repo, "show", "-s", "--format=%cI", PINNED_COMMIT],
    { encoding: "utf8" },
  ).trim();

  // One row per league-season: the metric values, and the matches behind them.
  const rows = [];
  for (const season of SEASONS) {
    for (const league of Object.keys(LEAGUES)) {
      let text;
      try {
        text = atPin(`${season}/${league}.json`);
      } catch (error) {
        const reason = error.stderr?.toString().trim() || error.message;
        console.error(`cannot read ${season}/${league}.json at ${PINNED_COMMIT}: ${reason}`);
        process.exit(1);
      }
      const matches = (JSON.parse(text).matches ?? []).filter(
        (m) => Array.isArray(m?.score?.ft) && m.score.ft.length === 2,
      );
      if (matches.length === 0) {
        console.error(`no played matches in ${season}/${league}.json`);
        process.exit(1);
      }
      const values = {};
      for (const metric of METRICS) {
        const sum = matches.reduce((acc, m) => acc + metric.of(m.score.ft[0], m.score.ft[1]), 0);
        values[metric.key] = sum / matches.length;
      }
      rows.push({ season, league, matches: matches.length, values });
    }
  }

  const totalMatches = rows.reduce((acc, r) => acc + r.matches, 0);

  const metrics = METRICS.map((metric) => {
    const values = rows.map((r) => r.values[metric.key]).sort((x, y) => x - y);
    const round = (v) => Number(v.toFixed(4));
    const low = percentile(values, 10);
    const high = percentile(values, 90);
    return {
      key: metric.key,
      label: metric.label,
      unit: metric.unit,
      low,
      high,
      observed: {
        min: round(values[0]),
        p10: round(percentile(values, 10)),
        median: round(percentile(values, 50)),
        p90: round(percentile(values, 90)),
        max: round(values[values.length - 1]),
        pooled_mean: round(
          rows.reduce((acc, r) => acc + r.values[metric.key] * r.matches, 0) / totalMatches,
        ),
      },
    };
  });

  const artifact = {
    source: {
      name: "openfootball/football.json",
      url: "https://github.com/openfootball/football.json",
      licence: "CC0-1.0",
      commit: PINNED_COMMIT,
      commit_date: commitDate,
    },
    population: {
      competitions: Object.values(LEAGUES),
      seasons: SEASONS,
      excluded_seasons: {
        "2019-20": "played behind closed doors; Ligue 1 abandoned at 279/380",
        "2020-21": "played largely behind closed doors",
      },
      league_seasons: rows.length,
      matches: totalMatches,
    },
    band_rule:
      "10th-90th percentile of the per-league-season values, stored at full " +
      "precision. Fixed before the first number was computed.",
    metrics,
  };

  const out = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    "..",
    "src-tauri",
    "crates",
    "sim-bench",
    "data",
    "big5-scorelines.json",
  );
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, `${JSON.stringify(artifact, null, 2)}\n`);

  console.log(`${rows.length} league-seasons, ${totalMatches} matches`);
  for (const m of metrics) {
    const fmt = (v) => (m.unit === "percent" ? `${v.toFixed(1)}%` : v.toFixed(2));
    console.log(
      `  ${m.label.padEnd(22)} band ${String(m.low).padStart(6)}–${String(m.high).padEnd(6)}` +
        `  observed ${fmt(m.observed.min)} … ${fmt(m.observed.max)}` +
        `  (median ${fmt(m.observed.median)}, pooled ${fmt(m.observed.pooled_mean)})`,
    );
  }
  console.log(`\nwrote ${path.relative(process.cwd(), out)}`);
}

main();
