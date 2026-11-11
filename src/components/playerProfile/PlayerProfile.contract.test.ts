import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import { terminationBlockedBy } from "./PlayerProfile.contract";

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
