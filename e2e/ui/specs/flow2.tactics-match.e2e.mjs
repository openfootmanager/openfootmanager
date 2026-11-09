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

/** Picks, in the native select `select`, the first option whose value differs from the current. */
async function chooseAnotherOption(select) {
  const current = await select.getValue();
  const values = await select.$$("option").map((option) => option.getAttribute("value"));
  const target = values.find((value) => value !== current);
  expect(target).toBeDefined();
  await select.selectByAttribute("value", target);
  await browser.waitUntil(async () => (await select.getValue()) === target, {
    timeout: 10_000,
    timeoutMsg: `the select never took the value ${target}`,
  });
  return target;
}

describe("flow 2: a tactics edit, then the match", () => {
  it("starts a career and reaches the dashboard", startCareerToDashboard);

  it("changes the formation and the play style on the Tactics screen", async () => {
    await clickWhenReady(buttonNamed("Tactics"));

    const formation = $('select[aria-label="Formation"]');
    await formation.waitForDisplayed({ timeout: 30_000 });
    edit.formation = await chooseAnotherOption(formation);

    const playStyle = $('select[aria-label="Play Style"]');
    await playStyle.waitForDisplayed({ timeout: 10_000 });
    await chooseAnotherOption(playStyle);
    // What the player reads for the style is the option's text, not its value.
    edit.playStyle = (await playStyle.$("option:checked").getText()).trim();
  });

  it("carries the edit to the pre-match screen of the first match day", async () => {
    await clickWhenReady(buttonNamed("Home"));
    await goToThePreMatchScreen();

    // The pre-match header names the side's formation and style, "4-4-2 · Defensive".
    await $(
      `//*[contains(normalize-space(.), "${edit.formation}") and contains(normalize-space(.), "${edit.playStyle}")]`,
    ).waitForDisplayed({ timeout: 30_000 });
  });

  it("starts the match, and it goes live", async () => {
    await clickWhenReady(buttonNamed("Start Match"));
    await $('//*[normalize-space(.)="Live"]').waitForDisplayed({ timeout: 90_000 });
  });
});
