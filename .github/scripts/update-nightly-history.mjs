#!/usr/bin/env node
// Maintains nightly-history.json on the nightly metadata branch: the last KEEP nightly builds,
// newest first. Each entry is that build's own nightly-release-manifest.json, plus what changed
// since the build before it. The website's download page reads it to list past nightlies.
//
//   node .github/scripts/update-nightly-history.mjs <history.json> <nightly-release-manifest.json>
//
// Run from a checkout with full history (actions/checkout fetch-depth: 0): the commit lists come
// from `git log <previous build>..<this build>`.
//
// The first time, when <history.json> does not exist yet, it is seeded from every nightly release
// already on GitHub (needs `gh` and GH_TOKEN), so there is no separate backfill step.
//
// Tolerant of runs arriving out of order or not at all: the previous build is the newest one
// published *before* this one, so a build whose metadata run was cancelled (the workflow cancels
// an older run when a newer one starts) simply has its commits counted in the next build's range.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const KEEP = 30;
export const MAX_COMMITS = 100;
const NIGHTLY_TAG = /^nightly-\d{8}-[0-9a-f]{7,40}$/;

/** The real repository, through the git CLI. Tests pass their own. */
export const gitCli = {
  hasCommit(sha) {
    try {
      execFileSync("git", ["cat-file", "-e", `${sha}^{commit}`], { stdio: "ignore" });
      return true;
    } catch {
      return false;
    }
  },
  commitsBetween(from, to) {
    return execFileSync("git", ["log", "--no-merges", "--format=%h%x09%s", `${from}..${to}`], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    })
      .split("\n")
      .filter(Boolean)
      .map((line) => {
        const tab = line.indexOf("\t");
        return { sha: line.slice(0, tab), subject: line.slice(tab + 1) };
      });
  },
};

const newestFirst = (a, b) => b.publishedAt.localeCompare(a.publishedAt);

/**
 * `builds` with `manifest` added (or replaced, if its tag is already there), newest first, at most
 * KEEP. Pure apart from the `git` lookups.
 */
export function addBuild(builds, manifest, git = gitCli) {
  if (!NIGHTLY_TAG.test(manifest.tag ?? "")) {
    throw new Error(`Not a per-build nightly tag: ${manifest.tag}`);
  }

  const others = builds.filter((build) => build.tag !== manifest.tag);
  const previous =
    others.filter((build) => build.publishedAt < manifest.publishedAt).sort(newestFirst)[0] ?? null;

  // Unknown when either end is missing from this checkout, rather than a guessed zero.
  let commitCount = null;
  let commits = [];
  if (previous && git.hasCommit(previous.commit) && git.hasCommit(manifest.commit)) {
    const all = git.commitsBetween(previous.commit, manifest.commit);
    commitCount = all.length;
    commits = all.slice(0, MAX_COMMITS);
  }

  const repository = `https://github.com/${manifest.repository}`;
  // The release notes are boilerplate the page never shows; leaving them out keeps the file small.
  const { notes: _notes, ...entry } = manifest;

  return [
    {
      ...entry,
      previousTag: previous?.tag ?? null,
      compareUrl: previous ? `${repository}/compare/${previous.tag}...${manifest.tag}` : null,
      checksumsUrl: `${repository}/releases/download/${manifest.tag}/nightly-checksums.txt`,
      commitCount,
      commits,
    },
    ...others,
  ]
    .sort(newestFirst)
    .slice(0, KEEP);
}

/** A history from scratch: every earlier manifest, oldest first, then this one. */
export function seedHistory(earlierManifests, manifest, git = gitCli) {
  let builds = [];
  for (const earlier of [...earlierManifests].sort((a, b) => -newestFirst(a, b))) {
    builds = addBuild(builds, earlier, git);
  }
  return addBuild(builds, manifest, git);
}

/** Every published per-build nightly already on GitHub, as its manifest. */
function releasedManifests(exceptTag) {
  const gh = (args) => execFileSync("gh", args, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  const releases = JSON.parse(
    gh(["release", "list", "--limit", "200", "--json", "tagName,isDraft"]),
  );
  const manifests = [];
  for (const { tagName, isDraft } of releases) {
    if (isDraft || tagName === exceptTag || !NIGHTLY_TAG.test(tagName)) continue;
    try {
      manifests.push(
        JSON.parse(
          gh(["release", "download", tagName, "-p", "nightly-release-manifest.json", "-O", "-"]),
        ),
      );
    } catch {
      console.warn(`Skipping ${tagName}: no nightly-release-manifest.json on the release.`);
    }
  }
  return manifests;
}

function main([historyPath, manifestPath]) {
  if (!historyPath || !manifestPath) {
    console.error(
      "usage: update-nightly-history.mjs <history.json> <nightly-release-manifest.json>",
    );
    process.exit(2);
  }

  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  let builds;

  if (existsSync(historyPath)) {
    const history = JSON.parse(readFileSync(historyPath, "utf8"));
    if (history.schemaVersion !== 1 || !Array.isArray(history.builds)) {
      throw new Error(`${historyPath} is not a schemaVersion 1 history file`);
    }
    builds = addBuild(history.builds, manifest);
  } else {
    const earlier = releasedManifests(manifest.tag);
    console.log(`Seeding the history from ${earlier.length} earlier nightly releases.`);
    builds = seedHistory(earlier, manifest);
  }

  const history = { schemaVersion: 1, updatedAt: new Date().toISOString(), builds };
  writeFileSync(historyPath, `${JSON.stringify(history, null, 2)}\n`);
  console.log(
    `${historyPath}: ${builds.length} builds, newest ${builds[0].tag} (${builds[0].commitCount ?? "no"} commits since the one before).`,
  );
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main(process.argv.slice(2));
}
