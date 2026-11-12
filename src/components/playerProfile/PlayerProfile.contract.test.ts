import { describe, expect, it } from "vitest";

import type { ActiveLoanData, PlayerData, StaffData } from "../../store/gameStore";
import { clubStateKey, terminationBlockedBy } from "./PlayerProfile.contract";

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

describe("clubStateKey", () => {
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
  const aide = (patch: Partial<StaffData>) =>
    ({ id: "s1", team_id: "mine", wage: 2_000, ...patch }) as StaffData;
  const loan = (patch: Partial<ActiveLoanData> = {}): ActiveLoanData => ({
    parent_team_id: "mine",
    loan_team_id: "rival",
    start_date: "2031-07-01",
    end_date: "2032-06-30",
    wage_contribution_pct: 50,
    ...patch,
  });
  const SQUAD = [member({ id: "a" }), member({ id: "b" })];
  const STAFF = [aide({ id: "s1" })];
  const TEAMS = [{ id: "mine", wage_budget: 900_000 }];
  const key = (
    patch: Partial<{
      players: PlayerData[];
      staff: StaffData[];
      teams: { id: string; wage_budget: number }[];
    }> = {},
  ) => clubStateKey({ players: SQUAD, staff: STAFF, teams: TEAMS, ...patch }, "mine");

  /**
   * Given the club's squad and staff, with a player and a staff member at another club
   * When the key is made with and without those two
   * Then it is the same: only what the club pays and fields is in it
   */
  it("ignores players and staff at other clubs", () => {
    const withOthers = key({
      players: [...SQUAD, member({ id: "x", team_id: "rival", wage: 99 })],
      staff: [...STAFF, aide({ id: "sx", team_id: "rival", wage: 99 })],
    });

    expect(withOthers).toBe(key());
  });

  /**
   * Given the club's squad, staff and wage budget
   * When a player is signed, released, injured, renewed or given a new contract end, a staff member
   *      is hired, fired or has his wage changed, or the wage budget changes
   * Then each gives a different key: the previews that depend on the whole club are asked again.
   *      Staff wages are in the wage bill (`calc_wages`), so a hire changes the projection though
   *      the playing squad did not change
   */
  it.each([
    ["a signing", { players: [...SQUAD, member({ id: "c" })] }],
    ["a release", { players: [SQUAD[0]] }],
    [
      "an injury",
      { players: [SQUAD[0], member({ id: "b", injury: { name: "Sprain", days_remaining: 9 } })] },
    ],
    ["a renewed wage", { players: [SQUAD[0], member({ id: "b", wage: 9_000 })] }],
    [
      "a new contract end",
      { players: [SQUAD[0], member({ id: "b", contract_end: "2034-06-30" })] },
    ],
    ["a staff hire", { staff: [...STAFF, aide({ id: "s2", wage: 3_000 })] }],
    ["a staff dismissal", { staff: [] }],
    ["a staff member's new wage", { staff: [aide({ id: "s1", wage: 4_000 })] }],
    ["a new wage budget", { teams: [{ id: "mine", wage_budget: 950_000 }] }],
  ])("changes with %s", (_what, patch) => {
    expect(key(patch)).not.toBe(key());
  });

  /**
   * Given a player of the club loaned out, whose wage the club (the parent) still pays a share of
   * When the loan is made, and again when its wage share changes
   * Then the key changes each time, though he is no longer at the club: his wage is still in the
   *      club's bill
   */
  it("changes with a loaned-out player the club still pays", () => {
    const away = (pct: number) =>
      member({
        id: "b",
        team_id: "rival",
        active_loan: loan({ wage_contribution_pct: pct }),
      });

    const loanedAt50 = key({ players: [SQUAD[0], away(50)] });

    expect(loanedAt50).not.toBe(key());
    expect(key({ players: [SQUAD[0], away(80)] })).not.toBe(loanedAt50);
  });

  /**
   * Given the same squad and staff listed in another order
   * When the key is made
   * Then it is the same: the order a game lists them in is not a change
   */
  it("does not depend on the order of the lists", () => {
    const staff = [aide({ id: "s1" }), aide({ id: "s2", wage: 3_000 })];

    expect(key({ players: [...SQUAD].reverse(), staff: [...staff].reverse() })).toBe(
      key({ staff }),
    );
  });

  /** Given no club (a manager with none), then there is no key to change. */
  it("is empty for a manager with no club", () => {
    expect(clubStateKey({ players: SQUAD, staff: STAFF, teams: TEAMS }, null)).toBe("");
  });
});
