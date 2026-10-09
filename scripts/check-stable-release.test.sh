#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
node --input-type=module <<'NODE'
import assert from "node:assert/strict";
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const workflow = readFileSync(
  ".github/workflows/tauri-action.yml",
  "utf8",
);
const nightly = readFileSync(".github/workflows/nightly-tauri-action.yml", "utf8");
const metadataScript = nightly.split("\n  prepare-nightly:")[1]
  ?.split("\n  # The same gauntlet")[0]?.split("        run: |\n")[1]?.replace(/^ {10}/gm, "");

function checkRelease(httpStatus, body, exitCode, options = {}) {
  const directory = mkdtempSync(join(tmpdir(), "ofm-release-state-"));
  try {
    // Isolate tool lookup: a missing fixture CLI must never fall through to real GitHub.
    symlinkSync(process.execPath, join(directory, "node"));
    symlinkSync("/bin/bash", join(directory, "bash"));
    symlinkSync("/usr/bin/dirname", join(directory, "dirname"));
    symlinkSync("/usr/bin/date", join(directory, "date"));
    if (options.gitError) {
      writeFileSync(join(directory, "git"), "#!/nonexistent/ofm-fixture-shell\n");
      chmodSync(join(directory, "git"), 0o755);
    } else {
      symlinkSync("/usr/bin/git", join(directory, "git"));
    }
    const gh = join(directory, "gh");
    writeFileSync(gh, options.spawnError ? "#!/nonexistent/ofm-fixture-shell\n" : `#!/usr/bin/env bash
printf '%s\\n' "$@" > "$FIXTURE_ARGS"
if [[ -n "$FIXTURE_HTTP" ]]; then
  printf 'HTTP/2.0 %s\\r\\nContent-Type: application/json\\r\\n\\r\\n%s\\n' "$FIXTURE_HTTP" "$FIXTURE_BODY"
fi
exit "$FIXTURE_EXIT"
`);
    chmodSync(gh, 0o755);
    mkdirSync(join(directory, "scripts"));
    mkdirSync(join(directory, "src-tauri"));
    copyFileSync("scripts/check-release-state.sh", join(directory, "scripts/check-release-state.sh"));
    const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
    if ("version" in options) config.version = options.version;
    writeFileSync(join(directory, "src-tauri/tauri.conf.json"), JSON.stringify(config));
    if (options.removeConfig) rmSync(join(directory, "src-tauri/tauri.conf.json"));
    const args = join(directory, "args");
    const output = join(directory, "output");
    writeFileSync(output, "");
    const env = {
      ...process.env,
      PATH: directory,
      GITHUB_REPOSITORY: options.repository ?? "fixture/project",
      FIXTURE_ARGS: args,
      FIXTURE_HTTP: String(httpStatus),
      FIXTURE_BODY: body,
      FIXTURE_EXIT: String(exitCode),
      GITHUB_SHA: "abc1234567890abcdef",
      GITHUB_OUTPUT: output,
      INPUT_RELEASE_TAG: options.releaseTag ?? "",
    };
    delete env.RELEASE_TAG;
    if ("releaseTag" in options) env.RELEASE_TAG = options.releaseTag;
    if (options.metadata) assert.ok(metadataScript, "the production metadata script must exist");
    const result = spawnSync("/bin/bash", options.metadata ? ["-c", metadataScript] : ["scripts/check-release-state.sh"], {
      cwd: directory,
      encoding: "utf8",
      env,
    });
    return { ...result, args: existsSync(args) ? readFileSync(args, "utf8") : "", output: readFileSync(output, "utf8") };
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

// Given a stable rerun, when jobs start, then release state is checked before any build or upload.
test("stable release state gates verification and the upload matrix", () => {
  const guard = workflow.split("\n  check-release-state:")[1]?.split("\n  verify:")[0];
  assert.ok(guard, "a pre-build release-state job must exist");
  assert.match(guard, /contents: read/);
  assert.match(guard, /GH_TOKEN: \$\{\{ secrets\.GITHUB_TOKEN \}\}/);
  assert.match(guard, /bash scripts\/check-release-state\.sh/);
  const verify = workflow.split("\n  verify:")[1]?.split("\n  publish-tauri:")[0];
  assert.match(verify, /needs: check-release-state/);
  assert.match(workflow.split("\n  publish-tauri:")[1], /needs: verify/);
});

// Given overlapping stable runs, when both target a draft, then one shared workflow group queues them.
test("stable runs serialize the guard through finalization without cancellation", () => {
  const header = workflow.split("\njobs:")[0];
  assert.match(header, /\nconcurrency:\n  group: stable-release\n  cancel-in-progress: false(?:\n|$)/);
});

function precedingUploadStep(source, job) {
  const upload = source.split(`\n  ${job}:`)[1];
  assert.ok(upload, `${job} must exist`);
  const steps = upload.split("\n      - ");
  const action = steps.findIndex(step => step.startsWith("uses: tauri-apps/tauri-action@"));
  assert.ok(action > 0, "the upload action must follow a guard step");
  return steps[action - 1];
}

// Given an individual stable matrix retry, when dependencies are reused, then state is checked at upload.
test("each stable upload rechecks the release immediately before tauri-action", () => {
  const guard = precedingUploadStep(workflow, "publish-tauri");
  assert.match(guard, /GH_TOKEN: \$\{\{ secrets\.GITHUB_TOKEN \}\}/);
  assert.match(guard, /run: bash scripts\/check-release-state\.sh/);
  assert.doesNotMatch(guard, /RELEASE_TAG:/);
});

// Given a nightly rerun or override, when uploading, then the already resolved tag is checked unchanged.
test("each nightly upload checks its resolved tag immediately before tauri-action", () => {
  const guard = precedingUploadStep(nightly, "publish-tauri-nightly");
  assert.match(guard, /GH_TOKEN: \$\{\{ secrets\.GITHUB_TOKEN \}\}/);
  assert.match(guard, /RELEASE_TAG: \$\{\{ needs\.prepare-nightly\.outputs\.tag \}\}/);
  assert.match(guard, /run: bash scripts\/check-release-state\.sh/);
});

// Given a nightly override targeting a stable tag, when scheduled, then it shares the stable queue without cancelling it.
test("stable-tag nightly overrides share stable serialization while normal nightlies retain cancellation", () => {
  const header = nightly.split("\njobs:")[0];
  assert.match(header, /group: \$\{\{ startsWith\(inputs\.release_tag, 'v'\) && 'stable-release' \|\| 'nightly-release' \}\}/);
  assert.match(header, /cancel-in-progress: \$\{\{ !startsWith\(inputs\.release_tag, 'v'\) \}\}/);
});

// Given raw override metadata, when preparation runs, then validation precedes outputs and verification.
test("nightly preparation validates its tag before publishing outputs or starting verification", () => {
  assert.ok(metadataScript, "the metadata script must exist");
  const guard = metadataScript.indexOf('RELEASE_TAG="$tag" bash scripts/check-release-state.sh');
  assert.ok(guard >= 0 && guard < metadataScript.indexOf('} >> "$GITHUB_OUTPUT"'));
  const verify = nightly.split("\n  verify:")[1]?.split("\n  publish-tauri-nightly:")[0];
  assert.match(verify, /needs: prepare-nightly/);
});

// Given hostile override data, when actual preparation runs, then it cannot inject a different output tag.
test("a newline override cannot inject a stable tag into nightly metadata outputs", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: "nightly-safe\ntag=v0.3.0", metadata: true });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid release tag/);
  assert.equal(result.args, "");
  assert.equal(result.output, "");
});

// Given a valid exact override, when preparation checks it, then the same value reaches action and finalization.
test("nightly metadata preserves a valid exact override after validation", () => {
  const tag = "nightly/retry+build.1";
  const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: tag, metadata: true });
  assert.equal(result.status, 0, result.stderr);
  assert.ok(result.output.startsWith(`tag=${tag}\n`));
});

// Given action aliases that select a different release, when guarded, then no misleading tag lookup is allowed.
for (const tag of ["refs/tags/v0.3.0", "v__VERSION__"]) {
  test(`the action tag alias ${tag} stops before the release lookup`, () => {
    const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: tag });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /exact release tag/);
    assert.equal(result.args, "");
  });
}

// Given Unicode edge whitespace that Git accepts but the action trims, when checked, then no different tag is authorized.
for (const tag of ["\u00a0v0.3.0", "v0.3.0\u2003"]) {
  test(`an override with surrounding Unicode whitespace ${JSON.stringify(tag)} stops before lookup`, () => {
    const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: tag });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /exact release tag/);
    assert.equal(result.args, "");
  });
}

// Given a leading nonbreaking space, when actual metadata preparation runs, then it cannot hide a stable target in the nightly group.
test("Unicode whitespace cannot inject a trimmed stable target into nightly metadata", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: "\u00a0v0.3.0", metadata: true });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /exact release tag/);
  assert.equal(result.args, "");
  assert.equal(result.output, "");
});

// Given an unpublished version, when its exact tag is absent, then a new draft may be built.
test("a missing release permits a new draft using the Tauri version", () => {
  const result = checkRelease(404, '{"message":"Not Found"}', 1);
  assert.equal(result.status, 0, result.stderr);
  const version = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
  assert.ok(result.args.includes(`repos/fixture/project/releases/tags/v${version}\n`));
  assert.ok(result.args.includes("GET\n"));
});

// Given an existing draft, when stable publication is retried, then the draft can be resumed.
test("an existing draft permits a retry", () => {
  const result = checkRelease(200, '{"draft":true}', 0);
  assert.equal(result.status, 0, result.stderr);
});

// Given prerelease and build metadata, when the draft is checked, then its complete tag is preserved.
test("a version with prerelease and build metadata uses its exact release tag", () => {
  const version = "0.4.0-rc.1+build.7";
  const result = checkRelease(200, '{"draft":true}', 0, { version });
  assert.equal(result.status, 0, result.stderr);
  assert.ok(decodeURIComponent(result.args).includes(`repos/fixture/project/releases/tags/v${version}\n`));
});

const nightlyTag = "nightly-20261006-abc1234";

// Given a resolved nightly tag, when absent, then the exact tag permits a new draft without rereading app metadata.
test("a missing nightly release permits its resolved tag without a Tauri config", () => {
  const result = checkRelease(404, '{"message":"Not Found"}', 1, { releaseTag: nightlyTag, removeConfig: true });
  assert.equal(result.status, 0, result.stderr);
  assert.ok(result.args.includes(`repos/fixture/project/releases/tags/${nightlyTag}\n`));
});

// Given an existing nightly draft, when rerun, then that exact draft can be resumed.
test("an existing nightly draft permits a retry of its resolved tag", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: nightlyTag });
  assert.equal(result.status, 0, result.stderr);
  assert.ok(result.args.includes(`repos/fixture/project/releases/tags/${nightlyTag}\n`));
});

// Given a published nightly or override, when rerun, then its public assets cannot be replaced.
test("an already published nightly tag stops before upload", () => {
  const result = checkRelease(200, '{"draft":false}', 0, { releaseTag: nightlyTag });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /already published/);
  assert.ok(result.args.includes(`repos/fixture/project/releases/tags/${nightlyTag}\n`));
});

// Given a failed nightly lookup, when state is unknown, then the resolved tag fails closed.
test("a failed nightly release lookup refuses its resolved tag", () => {
  const result = checkRelease(503, '{"message":"Unavailable"}', 1, { releaseTag: nightlyTag });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unable to check/);
  assert.ok(result.args.includes(`repos/fixture/project/releases/tags/${nightlyTag}\n`));
});

// Given a valid override with a slash and plus, when checked, then it remains one encoded API path segment.
test("a valid nightly override is encoded as one release-tag path segment", () => {
  const tag = "nightly/retry+build.1";
  const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: tag });
  assert.equal(result.status, 0, result.stderr);
  assert.ok(result.args.includes(`repos/fixture/project/releases/tags/${encodeURIComponent(tag)}\n`));
});

// Given an empty resolved tag, when checked, then it cannot fall back to a different stable release.
test("an empty nightly tag stops without a lookup or stable fallback", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: "" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid release tag/);
  assert.equal(result.args, "");
});

// Given hostile tag data, when validated, then Git's reference rules reject it before any API call.
test("a nightly tag containing a newline stops before the lookup", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { releaseTag: "nightly\nother-output=value" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid release tag/);
  assert.equal(result.args, "");
});

// Given an unavailable reference validator, when checked, then publication fails closed.
test("a reference validation process failure stops before the lookup", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { gitError: true });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unable to validate release tag/);
  assert.equal(result.args, "");
});

// Given an already public version, when dispatch or a same-version push runs, then it refuses uploads.
test("an already published release stops the workflow", () => {
  const result = checkRelease(200, '{"draft":false}', 0);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /already published/);
});

// Given an unavailable API, when state cannot be established, then it cannot masquerade as a new tag.
test("a release lookup failure stops the workflow", () => {
  const result = checkRelease(503, '{"message":"Unavailable"}', 1);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unable to check/);
});

// Given denied access, when the API rejects the lookup, then it cannot authorize publication.
test("a denied release lookup stops the workflow", () => {
  const result = checkRelease(403, '{"message":"Forbidden"}', 1);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unable to check/);
});

// Given a malformed success response, when draft state is absent, then publication stops.
test("a release response without a boolean draft state stops the workflow", () => {
  const result = checkRelease(200, '{"draft":"true"}', 0);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid release response/);
});

// Given invalid JSON from the API, when draft state cannot be decoded, then publication stops.
test("a malformed release response stops the workflow", () => {
  const result = checkRelease(200, "not JSON", 0);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid release response/);
});

// Given a transport failure, when no HTTP response arrives, then publication stops.
test("a transport failure stops the workflow", () => {
  const result = checkRelease("", "", 1);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unable to check/);
});

// Given an unusable CLI, when the lookup process cannot spawn, then publication stops.
test("a release lookup process failure stops the workflow", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { spawnError: true });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unable to check/);
});

// Given an invalid repository, when the guard starts, then no API request is made.
test("an invalid repository stops before the release lookup", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { repository: "invalid" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid repository or Tauri release version/);
  assert.equal(result.args, "");
});

// Given an invalid version, when the guard starts, then no API request is made.
test("an invalid Tauri version stops before the release lookup", () => {
  const result = checkRelease(200, '{"draft":true}', 0, { version: "bad/version" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid repository or Tauri release version/);
  assert.equal(result.args, "");
});

// Given platform uploads precede the AppImage check, when a new build uploads, then it stays private.
test("new stable releases stay drafts until AppImage validation finishes", () => {
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
test("stable finalization publishes the app version without a prerelease flag", () => {
  const finalize = workflow.split("\n  finalize-stable:")[1];
  assert.ok(finalize, "a finalization job must exist");
  assert.match(finalize, /require\('\.\/src-tauri\/tauri\.conf\.json'\)\.version/);
  assert.match(finalize, /gh release edit "v\$release_version" --draft=false --prerelease=false/);
});
NODE
