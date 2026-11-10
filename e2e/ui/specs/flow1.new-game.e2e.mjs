// Flow 1 of the real-UI suite: a new game, from the main menu to the pre-match screen.
//
// It stops at the pre-match lineup: playing the match is flow 2's business. Each `it` is one step
// of the player's path, so a red night names the step that broke.
import {
  buttonNamed,
  buttonStartingWith,
  clickWhenReady,
  typeInto,
  waitForRoute,
} from "../helpers/ui.mjs";

describe("flow 1: a new game, to the first match day", () => {
  it("opens on the main menu", async () => {
    await buttonNamed("New Game").waitForDisplayed({ timeout: 60_000 });
  });

  it("takes a manager's details", async () => {
    await clickWhenReady(buttonNamed("New Game"));
    await typeInto($("#create-manager-field-firstName input"), "Ada");
    await typeInto($("#create-manager-field-lastName input"), "Tester");

    const dob = $("#create-manager-field-dob");
    await typeInto(dob.$('[aria-label="Day"]'), "15");
    await clickWhenReady(dob.$("button"));
    await clickWhenReady($('//button[normalize-space(.)="March"]'));
    await typeInto(dob.$('[aria-label="Year"]'), "1985");

    await clickWhenReady($("#create-manager-field-nationality button"));
    await typeInto($('input[placeholder="Search nationalities..."]'), "Brazil");
    await clickWhenReady($('//button[.//span[@title="Brazil"]]'));
  });

  it("goes on to the world, and starts a career in it", async () => {
    await clickWhenReady(buttonNamed("Choose World"));
    // "Build your world": no package is installed, so a random world is generated.
    await clickWhenReady(buttonNamed("Next"));
    await clickWhenReady(buttonNamed("Start Career"));
    // The world is generated before the club list opens: the longest wait of the flow.
    await waitForRoute("/select-team", 300_000);
  });

  it("lets the player choose a club, and manage it", async () => {
    await $("//*[normalize-space(.)='Choose Your Club']").waitForDisplayed({ timeout: 60_000 });
    // A club is selected on arrival; the panel naming it is what the player confirms.
    await $("//*[normalize-space(.)='Selected Club']").waitForDisplayed({ timeout: 60_000 });
    await clickWhenReady(buttonStartingWith("Manage "));
    await waitForRoute("/dashboard");
    // The career is the one the player asked for: the sidebar names the manager just created.
    await $('//*[normalize-space(.)="Ada Tester"]').waitForDisplayed({ timeout: 30_000 });
  });

  it("skips to the first match day and reaches the pre-match lineup", async () => {
    await clickWhenReady($('[aria-label="Choose match mode"]'));
    await clickWhenReady($('//button[starts-with(normalize-space(.), "Skip to Match Day")]'));

    // A blocker (unread urgent mail, a contract to settle) can stop the skip: go on regardless.
    await browser.waitUntil(
      async () => {
        const anyway = buttonNamed("Continue Anyway");
        if (await anyway.isDisplayed()) await anyway.click();
        return (await $$('//*[normalize-space(.)="Match Day"]').length) > 0;
      },
      { timeout: 120_000, interval: 1000, timeoutMsg: "the skip never reached a match day" },
    );

    // The skip ends in a digest of what happened on the way; the match is behind it.
    await clickWhenReady(buttonNamed("Back to Dashboard"));
    await clickWhenReady($('//button[contains(., "Go to the Field")]'));
    await clickWhenReady(buttonNamed("Confirm"));
    await waitForRoute("/match");

    // The pre-match screen: the kick-off button, and the substitutes the manager can bring on.
    await buttonNamed("Start Match").waitForDisplayed({ timeout: 60_000 });
    const substitutes = await $$('[data-testid^="pre-match-bench-"]');
    expect(substitutes.length).toBeGreaterThan(0);
    // The formation panel counts each line "filled/needed", one cell per position: the goalkeeper's
    // cell reads 1/1 in any shape, and anything else means the XI has no keeper.
    const goalkeeperCount = $('//div[span[1][normalize-space(.)="GK"]]/span[2]');
    await goalkeeperCount.waitForDisplayed({ timeout: 10_000 });
    expect((await goalkeeperCount.getText()).trim()).toBe("1/1");
  });
});
