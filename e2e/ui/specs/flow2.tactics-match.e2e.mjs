// Flow 2 of the real-UI suite: a tactics edit, then the match.
//
// A new career is started (flow 1's path, shared in helpers/career.mjs), the formation and the play
// style are changed on the Tactics screen, and the first match day shows those changes on its
// pre-match screen; the match then starts and goes live. Each `it` is one step of the player's
// path, so a red night names the step that broke.
import { goToThePreMatchScreen, startCareerToDashboard } from "../helpers/career.mjs";
import { buttonNamed, clickWhenReady, xpathLiteral } from "../helpers/ui.mjs";

// What the Tactics screen was changed to, read back for the later steps.
const edit = { formation: "", playStyle: "" };

/**
 * The text of an element: what the driver renders, or the DOM's where the driver reports none.
 * WebKitWebDriver gives an empty string for these controls' buttons, which are text in a span.
 */
async function textOf(element) {
  return (
    (await element.getText()).trim() || String(await element.getProperty("textContent")).trim()
  );
}

/**
 * Opens the combobox named `label`, picks the first option whose text differs from the one shown,
 * and returns the text of the option picked once the combobox shows it.
 */
async function chooseAnotherOption(label) {
  const combobox = $(`button[role="combobox"][aria-label="${label}"]`);
  await combobox.waitForDisplayed({ timeout: 30_000 });
  const current = await textOf(combobox);
  expect(current).not.toBe("");

  await clickWhenReady(combobox);
  // The list is drawn after the click, and $$ does not wait for it.
  await $('[role="listbox"]').waitForDisplayed({ timeout: 10_000 });
  // A plain loop: WebdriverIO's element array does not take part in Promise.all(.map(...)).
  const options = await $$('[role="option"]');
  let picked = "";
  let pickedOption;
  const seen = [];
  for (const option of options) {
    const text = await textOf(option);
    seen.push(text);
    if (text !== "" && text !== current) {
      picked = text;
      pickedOption = option;
      break;
    }
  }
  if (picked === "") {
    throw new Error(
      `no ${label} option differs from "${current}": ${options.length} options, texts ${JSON.stringify(seen)}`,
    );
  }
  await clickWhenReady(pickedOption);

  await browser.waitUntil(async () => (await textOf(combobox)) === picked, {
    timeout: 10_000,
    timeoutMsg: `the ${label} control never showed ${picked}`,
  });
  return picked;
}

/**
 * The line under the player's own club name in the pre-match header, "4-3-3 · Possession".
 *
 * The header shows both sides, home on the left and away on the right, and either can be the
 * player's, so the side is found by name: the first tab is labelled with the player's club (the
 * opponent's tab follows it), and the line is the second paragraph of the block naming that club.
 * Searching the whole page would also read the opponent's line.
 */
async function ownSideLine() {
  const ownTab = $(
    '//button[starts-with(normalize-space(.), "Opponent")]/preceding-sibling::button[1]',
  );
  await ownTab.waitForDisplayed({ timeout: 30_000 });
  // The DOM's text: the tab is drawn in capitals by CSS, and the header's name is not.
  const club = String(await ownTab.getProperty("textContent")).trim();
  expect(club).not.toBe("");
  return $(`//header//div[p[1][normalize-space(.)=${xpathLiteral(club)}]]/p[2]`);
}

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

    // The player's own side of the pre-match header names its formation and style, and only that
    // side counts: the opponent's line is on the same page.
    const line = await ownSideLine();
    await line.waitForDisplayed({ timeout: 30_000 });
    let shown = "";
    await browser
      .waitUntil(
        async () => {
          shown = await textOf(line);
          const lower = shown.toLowerCase();
          return (
            lower.includes(edit.formation.toLowerCase()) &&
            lower.includes(edit.playStyle.toLowerCase())
          );
        },
        {
          timeout: 10_000,
          timeoutMsg: `the player's side never showed ${edit.formation} and ${edit.playStyle}`,
        },
      )
      .catch((error) => {
        throw new Error(`${error.message}; it shows "${shown}"`);
      });
  });

  it("starts the match, and it goes live", async () => {
    await clickWhenReady(buttonNamed("Start Match"));
    await $('//*[normalize-space(.)="Live"]').waitForDisplayed({ timeout: 90_000 });
  });
});
