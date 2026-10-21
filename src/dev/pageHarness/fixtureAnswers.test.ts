import { describe, expect, it } from "vitest";

import { readStyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { fixtureHandlers } from "./fixtureAnswers";
import { gameState, squad } from "./fixture.generated";
import { UNANSWERED } from "./harnessIpc";

const handlers = fixtureHandlers(readStyleGuideOptions(""), "en");
const club = gameState.manager.team_id;

describe("fixture answers", () => {
  /** Given the user's club, when its squad is asked for, then the fixture's squad comes back. */
  it("answers for the user's club", () => {
    expect(handlers.get_squad({ teamId: club })).toBe(squad);
  });

  /**
   * Given another club, when its squad, standing or finances are asked for, then the fixture
   * declines: it only holds the user's club, and must not pass it off as another's.
   */
  it("declines another club", () => {
    for (const command of ["get_squad", "get_club_league_standing", "get_finance_snapshot"]) {
      expect(handlers[command]({ teamId: "another-club" })).toBe(UNANSWERED);
    }
  });

  /** Given no team id, when the finances are asked for, then they are the user's club's, as in Rust. */
  it("treats a missing team id as the user's club for finances", () => {
    expect(handlers.get_finance_snapshot({ teamId: null })).not.toBe(UNANSWERED);
  });

  /**
   * Given player ids, one of them unknown, when contract stages are asked for, then only the
   * known players' stages come back, as Rust leaves an unknown id out.
   */
  it("answers contract stages only for the players asked about", () => {
    const [first] = squad;
    const stages = handlers.get_contract_warning_stages({ playerIds: [first.id, "nobody"] });
    expect(Object.keys(stages as object)).toEqual([first.id]);
  });
});
