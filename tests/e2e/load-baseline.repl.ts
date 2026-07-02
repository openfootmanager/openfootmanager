import { browser, $ } from "@wdio/globals";

// Interactive spec: the world load, manager creation, and team hire are
// all handled by `--mcp-auto-start` in wdio.conf.ts, so by the time the
// WebView paints, the game is already at a hired-manager dashboard. This
// spec just hands the running app off to the human via `browser.debug()`.
describe("Baseline world — interactive load", () => {
    it("halts on the pre-loaded dashboard for inspection", async function () {
        // Disable Mocha timeout — `browser.debug()` holds until Ctrl-D.
        this.timeout(0);

        await $("#root").waitForExist({ timeout: 30_000 });

        console.log(
            "\n[e2e] Baseline world loaded via --mcp-auto-start. " +
                "Inspect the app, then Ctrl-D to resume.\n",
        );
        await browser.debug();
    });
});
