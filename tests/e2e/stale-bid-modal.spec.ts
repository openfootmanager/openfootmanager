import { browser, $, expect } from "@wdio/globals";

// Regression guard for the stale-bidTarget bug in useTransferBidFlow.
//
// The hook stores `bidTarget` as a snapshot React useState value set
// once at modal open. Everything downstream — `activeBidOffer`,
// `hasExistingOffer`, the negotiation-history panel — is derived from
// that snapshot. After a bid, `onGameUpdate` refreshes the parent's
// gameState (the player now has a Pending transfer_offer), but the
// hook keeps reading from the frozen `bidTarget`, so nothing derived
// updates.
//
// Observable proxy: `TransferBidModal` shows the
// "transfers.resumeNegotiationHint" ("Talks are still live with this
// club.") only when `hasExistingOffer` is true. If `bidTarget` was
// live, submitting a bid that produces a Pending offer would flip
// `hasExistingOffer` from false → true and the hint would appear.
// With the bug present, `hasExistingOffer` stays false and the hint
// never renders even though talks *are* now live.
//
// Fixture: baseline.json. Target: Aneurin Morgan (mv €540,800). Bid
// €500K = 0.5 — comfortably below any accept threshold, comfortably
// above any hard-reject floor → produces a counter-offer (Pending).
describe("Stale bid modal — e2e", () => {
    it("shows the resume-negotiation hint after a counter-offer response", async function () {
        this.timeout(120_000);

        await $("#root").waitForExist({ timeout: 30_000 });

        const transfersTab = await $('button[aria-label="Transfers"]');
        await transfersTab.waitForClickable({ timeout: 15_000 });
        await transfersTab.click();

        const search = await $('input[placeholder="Search by name..."]');
        await search.waitForExist({ timeout: 10_000 });
        await search.setValue("Aneurin Morgan");
        await browser.pause(300);

        const bidButton = await $("button=Bid");
        await bidButton.waitForClickable({ timeout: 10_000 });
        await bidButton.click();

        // Control check: before any bid, no live talks exist, so the
        // hint must NOT be rendered. If this fails, the hint is
        // stuck-visible for reasons unrelated to the stale bug.
        const preHint = await $(
            '//*[contains(text(), "Talks are still live with this club")]',
        );
        await expect(preHint).not.toBeDisplayed();

        // Bid in the counter-offer zone: high enough that the engine
        // doesn't reject outright, low enough that it doesn't accept.
        // 0.5 (€500K) against a €540K market value hits the counter
        // window for this fixture.
        const amountInput = await $("#bid-amount");
        await amountInput.waitForExist({ timeout: 10_000 });
        await amountInput.setValue("0.5");

        const submit = await $("button=Submit Bid");
        await submit.waitForClickable({ timeout: 10_000 });
        await submit.click();

        // Give the response and the store update time to propagate.
        await browser.pause(1500);

        // A Pending offer now exists on the player. If the hook is
        // reading from a live source, `hasExistingOffer` is true and
        // the hint is visible. If `bidTarget` is a stale snapshot,
        // the hint stays hidden despite talks being live — the bug
        // this guard exists to catch.
        const postHint = await $(
            '//*[contains(text(), "Talks are still live with this club")]',
        );
        await postHint.waitForDisplayed({ timeout: 10_000 });
        await expect(postHint).toBeDisplayed();
    });
});
