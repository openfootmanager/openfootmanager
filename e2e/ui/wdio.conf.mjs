import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

import { assertIsolated } from "./helpers/isolation.mjs";

const artifacts = process.env.OFM_E2E_ARTIFACTS ?? path.resolve("artifacts");
let driver;

export const config = {
  runner: "local",
  specs: ["./specs/**/*.e2e.mjs"],
  maxInstances: 1,
  // tauri-driver speaks WebDriver on 4444 and starts the app named in the capabilities.
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  capabilities: [{ maxInstances: 1, "tauri:options": { application: process.env.OFM_E2E_APP } }],
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  // Longer than the longest wait inside a step: generating the world allows 300 s for the club
  // screen, and mocha must not cut a step off before its own wait gives up and names the cause.
  mochaOpts: { ui: "bdd", timeout: 360_000 },

  onPrepare() {
    assertIsolated();
    mkdirSync(artifacts, { recursive: true });
    driver = spawn("tauri-driver", [], { stdio: ["ignore", process.stdout, process.stderr] });
    // The driver has to be listening before the first session is requested.
    return new Promise((resolve) => setTimeout(resolve, 3000));
  },

  onComplete() {
    driver?.kill();
  },

  // What failed is what the nightly run keeps: a picture, and the markup the selectors saw.
  async afterTest(test, _context, { passed }) {
    if (passed) return;
    const name = test.title.replace(/[^a-z0-9]+/gi, "-").toLowerCase();
    await browser.saveScreenshot(path.join(artifacts, `${name}.png`)).catch(() => {});
    const source = await browser.getPageSource().catch(() => "");
    writeFileSync(path.join(artifacts, `${name}.html`), source);
  },
};
