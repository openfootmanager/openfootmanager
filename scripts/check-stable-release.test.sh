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

function checkRelease(httpStatus, body, exitCode, options = {}) {
  const directory = mkdtempSync(join(tmpdir(), "ofm-release-state-"));
  try {
    // Isolate tool lookup: a missing fixture CLI must never fall through to real GitHub.
    symlinkSync(process.execPath, join(directory, "node"));
    symlinkSync("/bin/bash", join(directory, "bash"));
    symlinkSync("/usr/bin/dirname", join(directory, "dirname"));
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
    copyFileSync("scripts/check-stable-release-state.sh", join(directory, "scripts/check-stable-release-state.sh"));
    const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
    if ("version" in options) config.version = options.version;
    writeFileSync(join(directory, "src-tauri/tauri.conf.json"), JSON.stringify(config));
    const args = join(directory, "args");
    const result = spawnSync("/bin/bash", ["scripts/check-stable-release-state.sh"], {
      cwd: directory,
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: directory,
        GITHUB_REPOSITORY: options.repository ?? "fixture/project",
        FIXTURE_ARGS: args,
        FIXTURE_HTTP: String(httpStatus),
        FIXTURE_BODY: body,
        FIXTURE_EXIT: String(exitCode),
      },
    });
    return { ...result, args: existsSync(args) ? readFileSync(args, "utf8") : "" };
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
  assert.match(guard, /bash scripts\/check-stable-release-state\.sh/);
  const verify = workflow.split("\n  verify:")[1]?.split("\n  publish-tauri:")[0];
  assert.match(verify, /needs: check-release-state/);
  assert.match(workflow.split("\n  publish-tauri:")[1], /needs: verify/);
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
