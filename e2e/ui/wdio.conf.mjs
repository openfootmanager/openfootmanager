import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

import { freePort, waitForOwnDriver } from "./helpers/driver.mjs";
import { assertIsolated } from "./helpers/isolation.mjs";

const artifacts = process.env.OFM_E2E_ARTIFACTS ?? path.resolve("artifacts");
let driver;

// The driver's ports are picked once, by the launcher, and handed to the workers through the
// environment (each worker loads this file again). Never 4444: a driver a developer left running
// there carries the developer's environment, and the app it starts would open on their screen.
async function portFrom(name) {
  if (!process.env[name]) process.env[name] = String(await freePort());
  return Number(process.env[name]);
}
const driverPort = await portFrom("OFM_E2E_DRIVER_PORT");
const nativePort = await portFrom("OFM_E2E_NATIVE_PORT");

export const config = {
  runner: "local",
  specs: ["./specs/**/*.e2e.mjs"],
  maxInstances: 1,
  // tauri-driver speaks WebDriver here and starts the app named in the capabilities.
  hostname: "127.0.0.1",
  port: driverPort,
  path: "/",
  capabilities: [{ maxInstances: 1, "tauri:options": { application: process.env.OFM_E2E_APP } }],
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  // Longer than every wait inside the longest step added together: starting a career clicks three
  // buttons (30 s each) and then allows 300 s for the club screen, and mocha must not cut the step
  // off before its own waits give up and name the cause.
  mochaOpts: { ui: "bdd", timeout: 480_000 },

  async onPrepare() {
    assertIsolated();
    mkdirSync(artifacts, { recursive: true });
    driver = spawn(
      "tauri-driver",
      ["--port", String(driverPort), "--native-port", String(nativePort)],
      { stdio: ["ignore", process.stdout, process.stderr] },
    );
    // Go on only when this very process is the one listening on the port the suite will use.
    await waitForOwnDriver(driver, driverPort);
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
