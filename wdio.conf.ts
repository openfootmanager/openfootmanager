import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

// tauri-driver bridges the WebDriver protocol to the Tauri WebView on Linux
// (WebKitGTK) and Windows (WebView2). Must be on PATH — see
// tests/e2e/README.md for install steps. We spawn one process per session
// and let each Mocha spec drive the same WebView instance.
let tauriDriver: ChildProcess | null = null;

// Every test session gets a fresh XDG_DATA_HOME so the app writes its saves
// into a temp dir instead of ~/.local/share. Tauri respects XDG_DATA_HOME
// via app_data_dir(), so no engine change is required for isolation.
let sessionDataDir: string | null = null;

const APP_BINARY =
    process.env.TAURI_APP_BINARY ??
    path.resolve("./src-tauri/target/debug/openfootmanager");

// CLI args passed to the app binary via tauri-driver. Uses the MCP
// auto-start hook (`--mcp-auto-start world_path,team_id`) to bypass the
// new-game menu entirely: the game boots straight into a hired-manager
// dashboard on the given team.
//
// The manager identity and team id are pinned from the fixture — the
// exported world contains a manager "Testing Tester" (AR) currently at
// Club Buenos Aires. Using the same identity keeps the boot state
// faithful to what was exported.
const FIXTURE_WORLD_PATH = path.resolve(
    "./tests/e2e/fixtures/baseline.json",
);
const FIXTURE_TEAM_ID = "dda1d67c-3bca-47c4-8c78-e5bd19d00886"; // Club Buenos Aires
const APP_ARGS = [
    "--mcp-port",
    "4455",
    "--mcp-auto-start",
    `${FIXTURE_WORLD_PATH},${FIXTURE_TEAM_ID}`,
    "--manager-name",
    "Testing",
    "--manager-last-name",
    "Tester",
    "--manager-nationality",
    "AR",
];

export const config: WebdriverIO.Config = {
    runner: "local",
    tsConfigPath: "./tsconfig.node.json",

    specs: ["./tests/e2e/**/*.spec.ts"],
    exclude: [],

    maxInstances: 1,

    capabilities: [
        {
            maxInstances: 1,
            "tauri:options": {
                application: APP_BINARY,
                args: APP_ARGS,
            },
        } as WebdriverIO.Capabilities,
    ],

    logLevel: "info",
    bail: 0,

    hostname: "127.0.0.1",
    port: 4444,

    waitforTimeout: 10_000,
    connectionRetryTimeout: 120_000,
    connectionRetryCount: 3,

    framework: "mocha",
    reporters: ["spec"],
    mochaOpts: {
        ui: "bdd",
        timeout: 60_000,
    },

    onPrepare() {
        sessionDataDir = mkdtempSync(path.join(tmpdir(), "ofm-e2e-"));
        process.env.XDG_DATA_HOME = sessionDataDir;
        console.log(`[e2e] XDG_DATA_HOME=${sessionDataDir}`);
    },

    beforeSession() {
        // Passing XDG_DATA_HOME through the env of the spawned driver so the
        // WebView inherits it. Without this, the app would use the caller's
        // real save directory.
        tauriDriver = spawn("tauri-driver", [], {
            stdio: ["ignore", "inherit", "inherit"],
            env: {
                ...process.env,
                XDG_DATA_HOME: sessionDataDir ?? process.env.XDG_DATA_HOME,
            },
        });
        // Without an error listener a missing tauri-driver on PATH crashes
        // the runner with an uncaught ENOENT. Surface a hint instead so the
        // README's install step is discoverable.
        tauriDriver.on("error", (err) => {
            console.error(
                `[e2e] failed to spawn tauri-driver: ${err.message}\n` +
                    `      Install it and ensure it is on PATH — see tests/e2e/README.md.`,
            );
        });
    },

    async afterSession() {
        const proc = tauriDriver;
        tauriDriver = null;
        if (!proc || proc.exitCode !== null) return;
        // Wait for the driver to fully exit before returning — otherwise
        // the WebDriver port (4444) can still be bound when the next
        // session tries to spawn, and the runner sees a stale-connection
        // error instead of a clean restart.
        await new Promise<void>((resolve) => {
            proc.once("exit", () => resolve());
            proc.kill();
        });
    },

    onComplete() {
        if (sessionDataDir) {
            rmSync(sessionDataDir, { recursive: true, force: true });
            sessionDataDir = null;
        }
    },
};
