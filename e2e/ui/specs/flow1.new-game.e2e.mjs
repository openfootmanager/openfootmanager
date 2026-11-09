// Flow 1 of the real-UI suite: a new game, from the main menu to the pre-match screen.
//
// It stops at the pre-match lineup: playing the match is flow 2's business. Each `it` is one step
// of the player's path, so a red night names the step that broke.
import {
  enterManagerDetails,
  goToThePreMatchScreen,
  manageTheSelectedClub,
  openMainMenu,
  startCareerInANewWorld,
} from "../helpers/career.mjs";

describe("flow 1: a new game, to the first match day", () => {
  it("opens on the main menu", openMainMenu);

  it("takes a manager's details", enterManagerDetails);

  it("goes on to the world, and starts a career in it", startCareerInANewWorld);

  it("lets the player choose a club, and manage it", manageTheSelectedClub);

  it("skips to the first match day and reaches the pre-match lineup", async () => {
    await goToThePreMatchScreen();

    // The pre-match screen: the kick-off button (waited for above), and the substitutes the
    // manager can bring on.
    const substitutes = await $$('[data-testid^="pre-match-bench-"]');
    expect(substitutes.length).toBeGreaterThan(0);
    // The formation panel counts each line "filled/needed", one cell per position: the goalkeeper's
    // cell reads 1/1 in any shape, and anything else means the XI has no keeper.
    const goalkeeperCount = $('//div[span[1][normalize-space(.)="GK"]]/span[2]');
    await goalkeeperCount.waitForDisplayed({ timeout: 10_000 });
    expect((await goalkeeperCount.getText()).trim()).toBe("1/1");
  });
});
