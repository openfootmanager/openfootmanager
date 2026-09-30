// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  addBuild,
  KEEP,
  MAX_COMMITS,
  seedHistory,
  updateHistory,
} from "./update-nightly-history.mjs";

const REPO = "openfootmanager/openfootmanager";

function manifest(date, sha, publishedAt, extra = {}) {
  return {
    schemaVersion: 1,
    repository: REPO,
    tag: `nightly-${date}-${sha}`,
    version: `0.3.0-nightly.${date}+${sha}`,
    commit: `${sha}${"0".repeat(33)}`,
    publishedAt,
    releaseUrl: `https://github.com/${REPO}/releases/tag/nightly-${date}-${sha}`,
    notes: "Rolling nightly build from `develop`.",
    assets: [],
    ...extra,
  };
}

/** A fake repository: every commit exists, and each range holds `perRange` commits. */
function fakeGit(perRange = 2, missing = []) {
  const ranges = [];
  return {
    ranges,
    hasCommit: (sha) => !missing.includes(sha),
    commitsBetween(from, to) {
      ranges.push([from.slice(0, 7), to.slice(0, 7)]);
      return Array.from({ length: perRange }, (_, i) => ({
        sha: `${to.slice(0, 6)}${i}`,
        subject: `fix(test): change ${i} before ${to.slice(0, 7)}`,
      }));
    },
  };
}

const first = manifest("20260923", "eca2db3", "2026-09-23T01:34:20Z");
const second = manifest("20260924", "ba81840", "2026-09-24T21:06:45Z");
const third = manifest("20260924", "8e01f97", "2026-09-24T21:39:04Z");

describe("addBuild", () => {
  it("records the commits since the build published before it", () => {
    const git = fakeGit(3);
    const [newest, older] = addBuild(addBuild([], first, git), second, git);

    expect(newest.tag).toBe(second.tag);
    expect(newest.previousTag).toBe(first.tag);
    expect(newest.compareUrl).toBe(
      `https://github.com/${REPO}/compare/${first.tag}...${second.tag}`,
    );
    expect(newest.checksumsUrl).toBe(
      `https://github.com/${REPO}/releases/download/${second.tag}/nightly-checksums.txt`,
    );
    expect(newest.commitCount).toBe(3);
    expect(newest.commits[0]).toEqual({
      sha: "ba81840",
      subject: "fix(test): change 0 before ba81840",
    });
    expect(git.ranges).toEqual([["eca2db3", "ba81840"]]);

    expect(older.previousTag).toBeNull();
    expect(older.commitCount).toBeNull();
  });

  it("keeps the manifest but drops its release notes", () => {
    const [entry] = addBuild([], first, fakeGit());

    expect(entry.version).toBe(first.version);
    expect(entry.assets).toEqual([]);
    expect(entry).not.toHaveProperty("notes");
  });

  it("links a late build in, and relinks the newer build past it", () => {
    const git = fakeGit();
    // The third build's run lands before the second's (the second was cancelled and re-run).
    const builds = addBuild(addBuild(addBuild([], first, git), third, git), second, git);

    expect(builds.map((b) => [b.tag, b.previousTag])).toEqual([
      [third.tag, second.tag],
      [second.tag, first.tag],
      [first.tag, null],
    ]);
    // Without the relink the third build keeps first..third, claiming the second build's commits.
    expect(builds[0].compareUrl).toBe(
      `https://github.com/${REPO}/compare/${second.tag}...${third.tag}`,
    );
    expect(builds[0].commits[0].subject).toBe("fix(test): change 0 before 8e01f97");
    expect(git.ranges.at(-2)).toEqual(["eca2db3", "ba81840"]);
    expect(git.ranges.at(-1)).toEqual(["ba81840", "8e01f97"]);
  });

  it("leaves the newer build alone when it already points at the added one", () => {
    const git = fakeGit();
    const builds = addBuild(addBuild(addBuild([], first, git), second, git), third, git);
    const before = git.ranges.length;

    addBuild(builds, second, git);

    // Only the re-added build is recomputed; the third already follows it.
    expect(git.ranges.length - before).toBe(1);
  });

  it("replaces a build that is added twice instead of listing it twice", () => {
    const git = fakeGit();
    const builds = addBuild(addBuild(addBuild([], first, git), second, git), second, git);

    expect(builds.map((b) => b.tag)).toEqual([second.tag, first.tag]);
  });

  it("says nothing about commits it cannot see, rather than claiming none", () => {
    const [entry] = addBuild(addBuild([], first, fakeGit()), second, fakeGit(2, [first.commit]));

    expect(entry.previousTag).toBe(first.tag);
    expect(entry.commitCount).toBeNull();
    expect(entry.commits).toEqual([]);
  });

  it(`lists at most ${MAX_COMMITS} commits but counts them all`, () => {
    const [entry] = addBuild(addBuild([], first, fakeGit()), second, fakeGit(250));

    expect(entry.commitCount).toBe(250);
    expect(entry.commits).toHaveLength(MAX_COMMITS);
  });

  it("limits commit subjects to 200 characters", () => {
    const longSubject = `<script>${"x".repeat(220)}</script>`;
    const git = {
      hasCommit: () => true,
      commitsBetween: () => [{ sha: "ba81840", subject: longSubject }],
    };
    const [entry] = addBuild(addBuild([], first, git), second, git);

    expect(entry.commitCount).toBe(1);
    expect(entry.commits[0].subject).toBe(longSubject.slice(0, 200));
  });

  it(`keeps the newest ${KEEP} builds`, () => {
    let builds = [];
    const tags = [];
    for (let day = 0; day < KEEP + 5; day++) {
      const publishedAt = new Date(Date.UTC(2026, 9, 1 + day)).toISOString();
      const date = publishedAt.slice(0, 10).replaceAll("-", "");
      const build = manifest(date, `abc${String(day).padStart(4, "0")}`, publishedAt);
      tags.push(build.tag);
      builds = addBuild(builds, build, fakeGit(1));
    }

    expect(builds).toHaveLength(KEEP);
    expect(builds.map((b) => b.tag)).toEqual(tags.slice(-KEEP).reverse());
  });

  it("refuses anything that is not a per-build nightly tag", () => {
    expect(() => addBuild([], { ...first, tag: "nightly" }, fakeGit())).toThrow(
      /per-build nightly/,
    );
    expect(() => addBuild([], { ...first, tag: "v0.2.0" }, fakeGit())).toThrow(/per-build nightly/);
  });
});

describe("updateHistory", () => {
  const earlier = new Date("2026-09-30T00:00:00Z");
  const later = new Date("2026-09-30T06:00:00Z");

  it("stamps the time when a build is added", () => {
    const history = updateHistory({ schemaVersion: 1, builds: [] }, first, fakeGit(), earlier);

    expect(history).toMatchObject({ schemaVersion: 1, updatedAt: earlier.toISOString() });
    expect(history.builds).toHaveLength(1);
  });

  it("keeps the old stamp when a re-run changes nothing, so the file is identical", () => {
    const git = fakeGit();
    const once = updateHistory(
      updateHistory({ schemaVersion: 1, builds: [] }, first, git, earlier),
      second,
      git,
      earlier,
    );
    const again = updateHistory(once, second, git, later);

    expect(JSON.stringify(again)).toBe(JSON.stringify(once));
  });

  it("keeps the oldest retained build's predecessor after 31 builds and a re-run", () => {
    const git = fakeGit(1);
    let history = { schemaVersion: 1, builds: [] };
    for (let day = 0; day < KEEP + 1; day++) {
      const publishedAt = new Date(Date.UTC(2026, 9, 1 + day)).toISOString();
      const date = publishedAt.slice(0, 10).replaceAll("-", "");
      history = updateHistory(
        history,
        manifest(date, `abc${String(day).padStart(4, "0")}`, publishedAt),
        git,
        earlier,
      );
    }

    const oldest = history.builds.at(-1);
    expect(oldest.previousTag).not.toBeNull();
    const again = updateHistory(history, oldest, git, later);

    expect(again.builds.at(-1)).toEqual(oldest);
    expect(JSON.stringify(again)).toBe(JSON.stringify(history));
  });
});

describe("seedHistory", () => {
  it("builds the history oldest first, whatever order the releases come in", () => {
    const git = fakeGit();
    const builds = seedHistory([second, first], third, git);

    expect(builds.map((b) => [b.tag, b.previousTag])).toEqual([
      [third.tag, second.tag],
      [second.tag, first.tag],
      [first.tag, null],
    ]);
    expect(git.ranges).toEqual([
      ["eca2db3", "ba81840"],
      ["ba81840", "8e01f97"],
    ]);
  });
});
