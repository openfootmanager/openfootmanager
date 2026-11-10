import { describe, expect, it } from "vitest";

import { gameState } from "./fixture.generated";
import { HARNESS_PROFILES } from "./harnessPage";
import { profileCase } from "./profileCases";

const USER_CLUB = gameState.manager.team_id;
const playerOf = (id: string) => gameState.players.find((player) => player.id === id);

// The answers follow `ofm_core::player_knowledge`: what the club may read of a player depends on how
// it knows him. These pin that the harness shows each state the way the game would.
describe("the player profile's cases", () => {
  /**
   * Given each state the club's knowledge of a player can be in
   * When its case is built
   * Then the five are different players (a state is never shown on another's player), and none is
   *      the player of another state's id
   */
  it("gives each state its own player", () => {
    const ids = HARNESS_PROFILES.map((variant) => profileCase(variant).playerId);
    expect(new Set(ids).size).toBe(HARNESS_PROFILES.length);
  });

  /**
   * Given the club's own player
   * When his case is built
   * Then he is at the user's club, known as its own, with his exact wage and the coaches' opinion
   */
  it("shows the own player's wage exactly, with the coaches' opinion", () => {
    const own = profileCase("own");
    const player = playerOf(own.playerId);

    expect(player?.team_id).toBe(USER_CLUB);
    expect(own.knowledge).toEqual({ kind: "own" });
    expect(own.wage).toEqual({ kind: "exact", weekly: player?.wage });
    expect(own.opinion?.source).toEqual({ kind: "coaches" });
    expect(own.scoutedReport).toBeNull();
  });

  /**
   * Given a player at another club of the user's division
   * When his case is built
   * Then he is known (live), his wage is hidden ("a known rival's wage is his club's business"),
   *      and no one has an opinion of his potential: knowing his rating is not knowing that
   */
  it("hides a known rival's wage and has no opinion of his potential", () => {
    const rival = profileCase("rival");
    const player = playerOf(rival.playerId);

    expect(player?.team_id).not.toBeNull();
    expect(player?.team_id).not.toBe(USER_CLUB);
    expect(rival.knowledge).toEqual({ kind: "known" });
    expect(rival.wage).toEqual({ kind: "hidden" });
    expect(rival.opinion).toBeNull();
    expect(rival.scoutedReport).toBeNull();
  });

  /**
   * Given a player a scout has reported on
   * When his case is built
   * Then the report's date is the one the knowledge, the opinion and the report all carry, his wage
   *      is rounded to two significant figures, and the opinion is the scout's
   */
  it("gives a scouted player a report, a rounded wage and the scout's opinion", () => {
    const scouted = profileCase("scouted");
    const player = playerOf(scouted.playerId);

    expect(scouted.knowledge.kind).toBe("scouted");
    const scoutedOn = scouted.knowledge.kind === "scouted" ? scouted.knowledge.scouted_on : "";
    expect(scouted.scoutedReport).toMatchObject({ player_id: scouted.playerId, scouted_on: scoutedOn });
    expect(scouted.opinion?.source).toMatchObject({ kind: "scout", scouted_on: scoutedOn });
    expect(scouted.wage).toEqual({
      kind: "approximate",
      weekly: Number((player?.wage ?? 0).toPrecision(2)),
    });
  });

  /**
   * Given a player the club knows nothing about
   * When his case is built
   * Then it holds no report, no opinion and no wage: public facts only
   */
  it("gives an unknown player public facts only", () => {
    const unknown = profileCase("unknown");

    expect(unknown.knowledge).toEqual({ kind: "unknown" });
    expect(unknown.wage).toEqual({ kind: "hidden" });
    expect(unknown.opinion).toBeNull();
    expect(unknown.scoutedReport).toBeNull();
    expect(unknown.positionRatings).toBeNull();
  });

  /**
   * Given a retired player, who is no longer in the world's players
   * When his case is built
   * Then he is fetched as a retiree, retired and clubless, and the knowledge answer leaves him out
   *      (an id no player has is left out of it), with no wage
   */
  it("gives a retired player as a retiree the knowledge answer leaves out", () => {
    const retired = profileCase("retired");

    expect(playerOf(retired.playerId)).toBeUndefined();
    expect(retired.retiredPlayer).toMatchObject({
      id: retired.playerId,
      retired: true,
      team_id: null,
    });
    expect(retired.knowledge).toBeNull();
    expect(retired.wage).toBeNull();
    expect(retired.opinion).toBeNull();
  });
});
