// Flow 2 of the real-UI suite: a tactics edit, then the match.
//
// A new career is started (flow 1's path, shared in helpers/career.mjs), the formation and the play
// style are changed on the Tactics screen, and the first match day shows those changes on its
// pre-match screen; the match then starts and goes live. Each `it` is one step of the player's
// path, so a red night names the step that broke.
import { goToThePreMatchScreen, startCareerToDashboard } from "../helpers/career.mjs";
import { buttonNamed, clickWhenReady } from "../helpers/ui.mjs";

// What the Tactics screen was changed to, read back for the later steps.
const edit = { formation: "", playStyle: "" };

/**
 * Opens the combobox named `label`, picks the first option whose text differs from the one shown,
 * and returns the text of the option picked once the combobox shows it.
 */
async function chooseAnotherOption(label) {
  const combobox = $(`button[role="combobox"][aria-label="${label}"]`);
  await combobox.waitForDisplayed({ timeout: 30_000 });
  const current = (await combobox.getText()).trim();

  await clickWhenReady(combobox);
  const options = await $$('[role="option"]');
  const texts = await Promise.all(options.map(async (option) => (await option.getText()).trim()));
  const index = texts.findIndex((text) => text !== "" && text !== current);
  expect(index).toBeGreaterThanOrEqual(0);
  const picked = texts[index];
  await clickWhenReady(options[index]);

  await browser.waitUntil(async () => (await combobox.getText()).trim() === picked, {
    timeout: 10_000,
    timeoutMsg: `the ${label} control never showed ${picked}`,
  });
  return picked;
}

/** An XPath condition: the element's text contains `text`, whatever its case. */
const containsIgnoringCase = (text) =>
  `contains(translate(normalize-space(.), "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz"), "${text.toLowerCase()}")`;

describe("flow 2: a tactics edit, then the match", () => {
  it("starts a career and reaches the dashboard", startCareerToDashboard);

  it("changes the formation and the play style on the Tactics screen", async () => {
    await clickWhenReady(buttonNamed("Tactics"));
    edit.formation = await chooseAnotherOption("Formation");
    edit.playStyle = await chooseAnotherOption("Play Style");
  });

  it("carries the edit to the pre-match screen of the first match day", async () => {
    // Without an edit there is nothing to carry, and every check below would pass on empty text.
    expect(edit.formation).not.toBe("");
    expect(edit.playStyle).not.toBe("");

    await clickWhenReady(buttonNamed("Home"));
    await goToThePreMatchScreen();

    // The pre-match header names the side's formation and style, "4-4-2 · Defensive".
    // Leaf elements only: an ancestor contains whatever its descendants say.
    await $(`//*[not(*) and contains(normalize-space(.), "${edit.formation}")]`).waitForDisplayed({
      timeout: 30_000,
    });
    await $(`//*[not(*) and ${containsIgnoringCase(edit.playStyle)}]`).waitForDisplayed({
      timeout: 10_000,
    });
  });

  it("starts the match, and it goes live", async () => {
    await clickWhenReady(buttonNamed("Start Match"));
    await $('//*[normalize-space(.)="Live"]').waitForDisplayed({ timeout: 90_000 });
  });
});
