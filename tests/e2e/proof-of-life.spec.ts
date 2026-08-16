import { $, expect } from "@wdio/globals";

describe("Openfoot Manager — e2e proof-of-life", () => {
    it("boots the Tauri window and mounts the React root", async () => {
        // index.html mounts the app into <div id="root">. If wdio can find
        // this element, the WebView loaded, Vite served the bundle, and
        // React rendered at least the outer container. That is the whole
        // point of this spec — the fixture/workflow-heavy specs come next.
        const root = await $("#root");
        await root.waitForExist({ timeout: 30_000 });
        await expect(root).toBeExisting();
    });
});
