// The player's path through a new game, one function per screen, shared by the flows.
//
// A flow that is not about starting a career still has to start one, and the screens it passes
// through are the same ones flow 1 checks. Keeping each step here means a changed screen is fixed
// once; flow 1 calls the steps one `it` at a time so a red night names the step that broke.
import {
  buttonNamed,
  buttonStartingWith,
  clickWhenReady,
  typeInto,
  waitForRoute,
} from "./ui.mjs";

export async function openMainMenu() {
  await buttonNamed("New Game").waitForDisplayed({ timeout: 60_000 });
}

export async function enterManagerDetails() {
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
}

export async function startCareerInANewWorld() {
  await clickWhenReady(buttonNamed("Choose World"));
  // "Build your world": no package is installed, so a random world is generated.
  await clickWhenReady(buttonNamed("Next"));
  await clickWhenReady(buttonNamed("Start Career"));
  // The world is generated before the club list opens: the longest wait of the flow.
  await waitForRoute("/select-team", 300_000);
}

export async function manageTheSelectedClub() {
  await $("//*[normalize-space(.)='Choose Your Club']").waitForDisplayed({ timeout: 60_000 });
  // A club is selected on arrival; the panel naming it is what the player confirms.
  await $("//*[normalize-space(.)='Selected Club']").waitForDisplayed({ timeout: 60_000 });
  await clickWhenReady(buttonStartingWith("Manage "));
  await waitForRoute("/dashboard");
}

/** Everything from the main menu to the dashboard of a fresh career. */
export async function startCareerToDashboard() {
  await openMainMenu();
  await enterManagerDetails();
  await startCareerInANewWorld();
  await manageTheSelectedClub();
}

/** From the dashboard to the pre-match screen of the first match day. */
export async function goToThePreMatchScreen() {
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
  await buttonNamed("Start Match").waitForDisplayed({ timeout: 60_000 });
}
