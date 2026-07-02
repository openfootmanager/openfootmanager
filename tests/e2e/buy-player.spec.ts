import { browser, $, expect } from "@wdio/globals";

// First real e2e assertion: bid on a transfer-listed player from another
// club, see the acceptance, and verify the player ends up in our squad.
//
// Fixture (see wdio.conf.ts): baseline.json loaded via --mcp-auto-start
// with manager Testing Tester at Club Buenos Aires
// (dda1d67c-3bca-47c4-8c78-e5bd19d00886). Their transfer_budget is
// €842k, so the target must be affordable at ~1.5× market value.
//
// Target: Aneurin Morgan — cheapest transfer-listed player in the
// fixture (Goalkeeper, market value €540,800). Bidding €820k is well
// above threshold and stays under the €842k transfer_budget.
describe("Buy a player — e2e", () => {
    it("bids on a listed player and finds him in the squad afterwards", async function () {
        this.timeout(120_000);

        await $("#root").waitForExist({ timeout: 30_000 });

        // 1. Navigate to Transfers tab. Each sidebar nav button has
        //    aria-label={label}, which is stable across collapsed vs
        //    expanded sidebar states.
        const transfersTab = await $('button[aria-label="Transfers"]');
        await transfersTab.waitForClickable({ timeout: 15_000 });
        await transfersTab.click();

        // 2. Filter the market list by name so exactly one Bid button
        //    shows.
        const search = await $('input[placeholder="Search by name..."]');
        await search.waitForExist({ timeout: 10_000 });
        await search.setValue("Aneurin Morgan");
        await browser.pause(300); // let the filter settle

        // 3. Click his Bid button.
        const bidButton = await $("button=Bid");
        await bidButton.waitForClickable({ timeout: 10_000 });
        await bidButton.click();

        // 4. Fill the amount. Input is <input id="bid-amount" type="number">
        //    and expects €M — the hook multiplies by 1_000_000 on submit.
        const amountInput = await $("#bid-amount");
        await amountInput.waitForExist({ timeout: 10_000 });
        await amountInput.setValue("0.82");

        // 5. Submit.
        const submit = await $("button=Submit Bid");
        await submit.waitForClickable({ timeout: 10_000 });
        await submit.click();

        // 6. Regression guard: the bid modal must NOT auto-close after
        //    acceptance. Wait longer than any legacy auto-close timer
        //    (was 2s), then assert the dialog is still on screen.
        //    Rejection / counter-offer paths already leave modals open
        //    until the user dismisses them; acceptance must behave the
        //    same way so the user has a chance to read the outcome.
        await browser.pause(3000);
        const dialog = await $('[role="dialog"]');
        await expect(dialog).toBeDisplayed();

        // 7. Dismiss the modal explicitly, as a user would.
        const closeButton = await $("button=Close");
        await closeButton.waitForClickable({ timeout: 5_000 });
        await closeButton.click();
        await dialog.waitForExist({ reverse: true, timeout: 5_000 });

        // 8. Navigate to Squad tab.
        const squadTab = await $('button[aria-label="Squad"]');
        await squadTab.waitForClickable({ timeout: 10_000 });
        await squadTab.click();

        // 9. Assert Aneurin Morgan is in the squad now. XPath because
        //    `*=` maps to "partial link text" in wdio and the player
        //    name is rendered in a non-link element.
        const player = await $('//*[contains(text(), "Aneurin Morgan")]');
        await player.waitForExist({ timeout: 15_000 });
        await expect(player).toBeExisting();

        // 10. After an incoming transfer, no two squad members should
        //     share a jersey number. The Squad roster is a <table>
        //     where each row's first <td> holds the jersey number (or
        //     "—" when unset). See
        //     src/components/squad/SquadRosterView.tsx:793-795.
        const jerseyNumbers: string[] = await browser.execute(() => {
            const rows = document.querySelectorAll("table tbody tr");
            return Array.from(rows)
                .map((row) => row.querySelector("td")?.textContent?.trim() ?? "")
                .filter((n) => n && n !== "—");
        });
        const seen = new Set<string>();
        const duplicates: string[] = [];
        for (const n of jerseyNumbers) {
            if (seen.has(n)) duplicates.push(n);
            seen.add(n);
        }
        await expect(duplicates).toEqual([]);

        // 11. After an accepted transfer, the user's inbox should contain
        //     the "Transfer Complete" message. See
        //     src-tauri/crates/ofm_core/src/messages.rs:158 and the
        //     i18n key be.msg.transferComplete.subject
        //     ("Transfer Complete: {{player}}").
        const inboxTab = await $('button[aria-label*="Inbox"]');
        await inboxTab.waitForClickable({ timeout: 10_000 });
        await inboxTab.click();
        const inboxEntry = await $(
            '//*[contains(text(), "Transfer Complete") and contains(text(), "Aneurin Morgan")]',
        );
        await inboxEntry.waitForExist({ timeout: 10_000 });
        await expect(inboxEntry).toBeExisting();
    });
});
