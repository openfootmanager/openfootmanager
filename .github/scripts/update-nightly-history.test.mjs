// @vitest-environment node
import { describe, expect, it } from "vitest";
import { addBuild, KEEP, MAX_COMMITS, seedHistory } from "./update-nightly-history.mjs";

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

  it("finds the previous build by date when runs arrive out of order", () => {
    const git = fakeGit();
    // The third build's run lands before the second's (the second was cancelled and re-run).
    const builds = addBuild(addBuild(addBuild([], first, git), third, git), second, git);

    expect(builds.map((b) => b.tag)).toEqual([third.tag, second.tag, first.tag]);
    expect(builds[1].previousTag).toBe(first.tag);
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
