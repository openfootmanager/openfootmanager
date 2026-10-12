// @vitest-environment node
import { afterEach, describe, expect, it, vi } from "vitest";
import { resolveConfig } from "vitest/node";

afterEach(() => {
  vi.unstubAllEnvs();
});

// Use Vitest's public resolver so a config-loading or override regression is observable without
// booting a second test pool. Each scenario reloads the real Vite config with its own CI flag.
async function resolveTestConfiguration(ci, options = {}) {
  vi.stubEnv("CI", ci);
  vi.stubEnv("VITEST_MAX_WORKERS", undefined);
  const { vitestConfig } = await resolveConfig(options);
  return {
    workers: vitestConfig.maxWorkers,
    timeout: vitestConfig.testTimeout,
    retries: vitestConfig.retry,
  };
}

describe("frontend worker policy", () => {
  /** Given a local run, when the repo config is resolved, then the pool uses two workers. */
  it("keeps local test runs within two workers", async () => {
    const configured = await resolveTestConfiguration("");
    const defaults = await resolveTestConfiguration("", { config: false });
    expect(configured).toEqual({ ...defaults, workers: 2 });
  });

  /** Given CI=false, when the repo config is resolved, then it still uses the local cap. */
  it("treats an explicitly disabled CI flag as a local run", async () => {
    expect((await resolveTestConfiguration("false")).workers).toBe(2);
  });

  /** Given CI, when the repo config is resolved, then Vitest's existing defaults are preserved. */
  it("preserves the existing CI pool and failure deadlines", async () => {
    const configured = await resolveTestConfiguration("true");
    const defaults = await resolveTestConfiguration("true", { config: false });
    expect(configured).toEqual(defaults);
  });

  /** Given a local worker override, when Vitest resolves it, then that explicit choice wins. */
  it("honours an explicit worker override", async () => {
    expect((await resolveTestConfiguration("", { maxWorkers: 3 })).workers).toBe(3);
  });
});
