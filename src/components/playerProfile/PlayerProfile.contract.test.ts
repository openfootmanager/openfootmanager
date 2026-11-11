import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import { squadStateKey, terminationBlockedBy } from "./PlayerProfile.contract";

const player = (patch: Partial<PlayerData>) =>
  ({ contract_end: "2033-06-30", active_loan: null, ...patch }) as PlayerData;

describe("terminationBlockedBy", () => {
  /** Given a player with a contract and no loan, then nothing stops releasing him. */
  it("lets a player with a contract be released", () => {
    expect(terminationBlockedBy(player({}))).toBeNull();
  });

  /** Given a player with no contract end, then there is no contract to end. */
  it("blocks a player with no contract end", () => {
    expect(terminationBlockedBy(player({ contract_end: null }))).toBe("no_contract");
  });

  /**
   * Given a player on loan at another club
   * Then he cannot be released: the backend refuses it too (`playerOnActiveLoan`), so the sheet and
   *      the actions menu say so up front
   */
  it("blocks a player who is on loan", () => {
    expect(
      terminationBlockedBy(
        player({
          active_loan: {
            parent_team_id: "a",
            loan_team_id: "b",
            start_date: "2031-07-01",
            end_date: "2032-06-30",
            wage_contribution_pct: 50,
            buy_option_fee: null,
          },
        }),
      ),
    ).toBe("on_loan");
  });

  /** Given a player with neither a contract end nor ... a loan on top, then the missing contract is said first. */
  it("names the missing contract before the loan", () => {
    expect(
      terminationBlockedBy(
        player({
          contract_end: null,
          active_loan: {
            parent_team_id: "a",
            loan_team_id: "b",
            start_date: "2031-07-01",
            end_date: "2032-06-30",
            wage_contribution_pct: 50,
            buy_option_fee: null,
          },
        }),
      ),
    ).toBe("no_contract");
  });
});

describe("squadStateKey", () => {
  const member = (patch: Partial<PlayerData>) =>
    ({
      id: "a",
      team_id: "mine",
      wage: 5_000,
      contract_end: "2033-06-30",
      injury: null,
      active_loan: null,
      ...patch,
    }) as PlayerData;
  const SQUAD = [member({ id: "a" }), member({ id: "b" })];
  const key = (players: PlayerData[]) => squadStateKey(players, "mine");

  /**
   * Given the club's squad and a player at another club
   * When the key is made with and without the other club's player
   * Then it is the same: only the club's own squad is in it
   */
  it("ignores players at other clubs", () => {
    expect(key([...SQUAD, member({ id: "x", team_id: "rival", wage: 99 })])).toBe(key(SQUAD));
  });

  /**
   * Given the club's squad
   * When another player is signed, one is released, one is injured, a wage is renewed, or a
   *      contract end changes
   * Then each gives a different key: the previews that depend on the whole club are asked again
   */
  it.each([
    ["a signing", [...SQUAD, member({ id: "c" })]],
    ["a release", [SQUAD[0]]],
    ["an injury", [SQUAD[0], member({ id: "b", injury: { name: "Sprain", days_remaining: 9 } })]],
    ["a renewed wage", [SQUAD[0], member({ id: "b", wage: 9_000 })]],
    ["a new contract end", [SQUAD[0], member({ id: "b", contract_end: "2034-06-30" })]],
  ])("changes with %s", (_what, players) => {
    expect(key(players)).not.toBe(key(SQUAD));
  });

  /**
   * Given the same squad listed in another order
   * When the key is made
   * Then it is the same: the order a game lists players in is not a change
   */
  it("does not depend on the order of the list", () => {
    expect(key([...SQUAD].reverse())).toBe(key(SQUAD));
  });
});
