#!/usr/bin/env node
// Loads the dev style guide and the page harness's pages in headless Chrome at the windows,
// scales and languages that have broken layouts before, and reports any page that scrolls
// sideways, reaches past the window's right edge, asks for a command the fixture can't answer,
// or renders nothing. Round 2's layout rule: no sideways scroll at 960×640, at any UI scale.
//
//   npm run dev                              # in another terminal
//   node scripts/page-layout-check.mjs [http://localhost:1420]
//
// Exits 1 when a case fails. Not a CI gate yet: it needs Chrome and a running dev server. It also
// needs Node 22 or later, for the built-in WebSocket (the project otherwise runs on Node 18+); on an
// older Node it says so and exits rather than adding a dependency for a dev-only tool.

import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const WINDOWS = [
  { width: 1280, height: 800, scale: "normal", lang: "en" },
  { width: 960, height: 640, scale: "normal", lang: "en" },
  // The reviewer's worst case for #956: German labels at extra-large scale in the minimum window.
  { width: 960, height: 640, scale: "xlarge", lang: "de" },
];

/** Every page at every window; `page: undefined` is the style guide itself. */
export const CASES = [undefined, "home", "squad"].flatMap((page) =>
  WINDOWS.map((window) => ({ page, ...window })),
);

/** Why this Node can't run the check, or null when it can. */
export function nodeProblem(webSocket, version) {
  return typeof webSocket === "function"
    ? null
    : `page-layout-check needs Node 22 or later for the built-in WebSocket; you have ${version}`;
}

/** What is wrong with one measured case; empty when it passes. */
export function judge({ scrollWidth, innerWidth, unanswered, pastEdge, rendered }) {
  const problems = [];
  if (scrollWidth > innerWidth) {
    problems.push(`the page scrolls sideways (${scrollWidth} > ${innerWidth})`);
  }
  if (pastEdge.length > 0) problems.push(`past the right edge: ${pastEdge.join(", ")}`);
  if (unanswered.length > 0) problems.push(`unanswered commands: ${unanswered.join(", ")}`);
  if (!rendered) problems.push("nothing rendered");
  return problems;
}

const MEASURE = `(async () => {
  await new Promise((resolve) => setTimeout(resolve, 2500));
  const harness = window.__pageHarness;
  return JSON.stringify({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
    unanswered: harness ? harness.unanswered : [],
    pastEdge: harness ? harness.pastEdge() : [],
    rendered: (document.getElementById("root")?.childElementCount ?? 0) > 0,
  });
})()`;

function caseUrl(base, { page, scale, lang }) {
  const query = new URLSearchParams({ theme: "light", scale, lang });
  if (page) query.set("page", page);
  return `${base}/style-guide.html?${query}`;
}

/**
 * The page's DevTools socket. Chrome was asked for any free port (0) and writes the one it took to
 * DevToolsActivePort in its profile, so two runs at once never collide on a fixed port.
 */
async function devtools(profile, chrome) {
  const portFile = join(profile, "DevToolsActivePort");
  for (let attempt = 0; attempt < 75; attempt += 1) {
    if (chrome.startError) throw chrome.startError;
    if (chrome.exitCode !== null) throw new Error(`Chrome exited (${chrome.exitCode}) on start`);
    if (existsSync(portFile)) {
      const [port] = readFileSync(portFile, "utf-8").split("\n");
      try {
        const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
        const page = targets.find((target) => target.type === "page");
        if (page) return page.webSocketDebuggerUrl;
      } catch (error) {
        // The port file can appear a moment before the endpoint answers: say so, then retry.
        console.error(`waiting for Chrome's DevTools: ${error.message}`);
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error("Chrome's DevTools endpoint never came up");
}

/** Stops Chrome's whole process group; one that has already gone is not an error. */
function stopChrome(chrome) {
  try {
    process.kill(-chrome.pid);
  } catch (error) {
    if (error.code !== "ESRCH") throw error;
  }
}

function connect(url) {
  const socket = new WebSocket(url);
  let nextId = 0;
  const pending = new Map();
  socket.addEventListener("message", (event) => {
    const reply = JSON.parse(event.data);
    pending.get(reply.id)?.(reply);
    pending.delete(reply.id);
  });
  const call = (method, params = {}) =>
    new Promise((resolve) => {
      nextId += 1;
      pending.set(nextId, resolve);
      socket.send(JSON.stringify({ id: nextId, method, params }));
    });
  return new Promise((resolve) => socket.addEventListener("open", () => resolve({ socket, call })));
}

async function measure(call, base, testCase) {
  await call("Emulation.setDeviceMetricsOverride", {
    width: testCase.width,
    height: testCase.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await call("Page.navigate", { url: caseUrl(base, testCase) });
  await new Promise((resolve) => setTimeout(resolve, 3000));
  const reply = await call("Runtime.evaluate", {
    expression: MEASURE,
    awaitPromise: true,
    returnByValue: true,
  });
  return JSON.parse(reply.result.result.value);
}

async function main(base) {
  const profile = mkdtempSync(join(tmpdir(), "page-layout-check-"));
  const chrome = spawn(
    "google-chrome",
    [
      "--headless=new",
      "--disable-gpu",
      "--no-sandbox",
      "--remote-debugging-port=0",
      `--user-data-dir=${profile}`,
      "about:blank",
    ],
    // Its own process group, so stopping it also stops the helper processes that write the profile.
    { stdio: "ignore", detached: true },
  );
  // Listen at once: a failed start (no google-chrome on PATH) is reported, not thrown unhandled.
  const exited = new Promise((resolve) => chrome.once("exit", resolve));
  chrome.once("error", (error) => {
    chrome.startError = error;
  });
  let failed = 0;
  try {
    const { socket, call } = await connect(await devtools(profile, chrome));
    for (const testCase of CASES) {
      const problems = judge(await measure(call, base, testCase));
      const label = `${testCase.page ?? "style guide"} ${testCase.width}×${testCase.height} ${testCase.scale} ${testCase.lang}`;
      console.log(
        `${problems.length === 0 ? "ok  " : "FAIL"} ${label}${problems.length ? `: ${problems.join("; ")}` : ""}`,
      );
      if (problems.length > 0) failed += 1;
    }
    socket.close();
  } finally {
    if (chrome.pid !== undefined && chrome.exitCode === null) {
      stopChrome(chrome);
      await exited;
    }
    rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  }
  console.log(`${CASES.length - failed} of ${CASES.length} cases pass`);
  process.exitCode = failed > 0 ? 1 : 0;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const problem = nodeProblem(globalThis.WebSocket, process.version);
  if (problem) {
    console.error(problem);
    process.exit(1);
  }
  await main(process.argv[2] ?? "http://localhost:1420");
}
