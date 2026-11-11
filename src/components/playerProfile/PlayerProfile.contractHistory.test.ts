import { describe, expect, it } from "vitest";

import type { PlayerMovementEntry } from "../../store/gameStore";
import { buildContractHistory } from "./PlayerProfile.contractHistory";

const TODAY = "2031-07-10";

function signing(
  date: string,
  club: string,
  contract: { start: string | null; end: string | null; weekly_wage: number },
  kind: PlayerMovementEntry["kind"] = "permanent_transfer",
  source: "transfer" | "renewal" | "initial" | "legacy_migrated" | "free_agent" = "transfer",
): PlayerMovementEntry {
  return { date, kind, to_team_id: club, to_team_name: club, contract: { ...contract, source } };
}

describe("buildContractHistory", () => {
  /**
   * Given a signing and a renewal
   * When the history is built
   * Then the contracts are listed newest first, each with its club, span and wage, and only the
   *      last is the one in force
   */
  it("lists the contracts newest first and marks the one in force", () => {
    const rows = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2031-06-30",
          weekly_wage: 7000,
        }),
        signing(
          "2031-07-01",
          "North FC",
          { start: "2031-07-01", end: "2033-06-30", weekly_wage: 10000 },
          "renewal",
          "renewal",
        ),
      ],
      TODAY,
    );

    expect(rows.map((row) => row.end)).toEqual(["2033-06-30", "2031-06-30"]);
    expect(rows.map((row) => row.inForce)).toEqual([true, false]);
    expect(rows[0]).toMatchObject({ clubName: "North FC", weeklyWage: 10000, source: "renewal" });
  });

  /**
   * Given a contract closed by a release before its planned end, and one that ran its full course
   * When the history is built
   * Then each says how it ended, and a closed contract is never the one in force, whatever its
   *      planned end
   */
  it("says how a contract was closed by a release, and never calls it in force", () => {
    const terminated = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2033-06-30",
          weekly_wage: 7000,
        }),
        {
          date: "2031-01-10",
          kind: "released",
          from_team_id: "North FC",
          release_reason: "terminated",
        },
      ],
      TODAY,
    );
    const expired = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2031-06-30",
          weekly_wage: 7000,
        }),
        {
          date: "2031-06-30",
          kind: "released",
          from_team_id: "North FC",
          release_reason: "expired",
        },
      ],
      TODAY,
    );

    expect(terminated[0]).toMatchObject({ endedBy: "terminated", inForce: false });
    expect(expired[0]).toMatchObject({ endedBy: "expired", inForce: false });
  });

  /**
   * Given a contract renewed before it ran out, so the two spans overlap
   * When the history is built
   * Then the older one records the day the renewal took over, and keeps its own recorded end
   */
  it("records the day an early renewal took over from the older contract", () => {
    const rows = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2032-06-30",
          weekly_wage: 7000,
        }),
        signing(
          "2031-01-15",
          "North FC",
          { start: "2031-01-15", end: "2034-06-30", weekly_wage: 9000 },
          "renewal",
          "renewal",
        ),
      ],
      TODAY,
    );

    expect(rows[1]).toMatchObject({ end: "2032-06-30", replacedOn: "2031-01-15" });
    expect(rows[0].replacedOn).toBeNull();
  });

  /**
   * Given a contract from a save made before starts were recorded
   * When the history is built
   * Then its start stays null: an unknown is never replaced by a date
   */
  it("keeps an unknown start unknown", () => {
    const [row] = buildContractHistory(
      [
        signing(
          "2029-07-01",
          "North FC",
          { start: null, end: "2033-06-30", weekly_wage: 7000 },
          "initial_contract",
          "legacy_migrated",
        ),
      ],
      TODAY,
    );

    expect(row.start).toBeNull();
    expect(row.source).toBe("legacy_migrated");
  });

  /**
   * Given a release with no contract before it, and a ledger with no contract at all
   * When the history is built
   * Then there are no rows and nothing fails
   */
  it("lists nothing for a ledger with no contract", () => {
    expect(
      buildContractHistory(
        [
          {
            date: "2031-06-30",
            kind: "released",
            from_team_id: "North FC",
            release_reason: "expired",
          },
        ],
        TODAY,
      ),
    ).toEqual([]);
    expect(buildContractHistory([], TODAY)).toEqual([]);
  });

  /**
   * Given two contracts recorded on the same day
   * When the history is built
   * Then the later recorded is listed above the earlier, as the movement list does
   */
  it("lists the later recorded of two same-day contracts first", () => {
    const rows = buildContractHistory(
      [
        signing("2031-07-01", "North FC", {
          start: "2031-07-01",
          end: "2032-06-30",
          weekly_wage: 1000,
        }),
        signing(
          "2031-07-01",
          "North FC",
          { start: "2031-07-01", end: "2034-06-30", weekly_wage: 2000 },
          "renewal",
          "renewal",
        ),
      ],
      TODAY,
    );

    expect(rows.map((row) => row.weeklyWage)).toEqual([2000, 1000]);
  });

  /**
   * Given a contract whose end is before today and no release after it
   * When the history is built
   * Then it is not in force (the player is between contracts)
   */
  it("does not call a contract past its end in force", () => {
    const [row] = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2031-06-30",
          weekly_wage: 7000,
        }),
      ],
      TODAY,
    );

    expect(row.inForce).toBe(false);
  });

  /**
   * Given a transfer from another club and a renewal at the new one
   * When the history is built
   * Then each row carries its club's id (for a link), and the transfer's the club he came from
   */
  it("carries the club's id and the club a transfer came from", () => {
    const rows = buildContractHistory(
      [
        {
          date: "2029-07-01",
          kind: "permanent_transfer",
          from_team_id: "south",
          from_team_name: "South FC",
          to_team_id: "north",
          to_team_name: "North FC",
          contract: {
            start: "2029-07-01",
            end: "2031-06-30",
            weekly_wage: 7000,
            source: "transfer",
          },
        },
        signing(
          "2031-07-01",
          "North FC",
          { start: "2031-07-01", end: "2033-06-30", weekly_wage: 10000 },
          "renewal",
          "renewal",
        ),
      ],
      TODAY,
    );

    expect(rows[1]).toMatchObject({ clubId: "north", fromClubName: "South FC" });
    expect(rows[0]).toMatchObject({ clubId: "North FC", fromClubName: null });
  });

  /**
   * Given a release dated 2030, and after it (in the ledger's append order) a contract migrated
   *       from an old save, whose start is unknown (an empty date) and which ends in 2033
   * When the history is built
   * Then the migrated contract is the one in force, and only the contract before the release is
   *      closed: the ledger's current contract is the last one appended, never the latest by date
   */
  it("keeps a contract migrated after a release in force", () => {
    const rows = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2031-06-30",
          weekly_wage: 7000,
        }),
        {
          date: "2030-06-30",
          kind: "released",
          from_team_id: "North FC",
          release_reason: "terminated",
        },
        {
          date: "",
          kind: "initial_contract",
          to_team_id: "North FC",
          to_team_name: "North FC",
          contract: {
            start: null,
            end: "2033-06-30",
            weekly_wage: 9000,
            source: "legacy_migrated",
          },
        },
      ],
      TODAY,
    );

    expect(rows.map((row) => [row.source, row.inForce, row.endedBy])).toEqual([
      ["legacy_migrated", true, null],
      ["transfer", false, "terminated"],
    ]);
  });

  /**
   * Given a contract to 2033 and a retirement in 2031
   * When the history is built
   * Then the retirement closes it, as the ledger does: it is not in force, and it says he retired
   */
  it("closes a contract when the player retires", () => {
    const rows = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2033-06-30",
          weekly_wage: 7000,
        }),
        { date: "2031-06-30", kind: "retired", from_team_id: "North FC" },
      ],
      TODAY,
    );

    expect(rows[0]).toMatchObject({ inForce: false, endedBy: "retired" });
  });

  /**
   * Given a contract terminated in 2030 and, in 2031, a free-agent signing that starts before the
   *       old contract's planned end
   * When the history is built
   * Then the terminated contract says it was terminated and never that it was "replaced": nothing
   *      took over from a contract that was already closed
   */
  it("does not say a closed contract was replaced by a later free-agent signing", () => {
    const rows = buildContractHistory(
      [
        signing("2029-07-01", "North FC", {
          start: "2029-07-01",
          end: "2033-06-30",
          weekly_wage: 7000,
        }),
        {
          date: "2030-03-01",
          kind: "released",
          from_team_id: "North FC",
          release_reason: "terminated",
        },
        signing(
          "2031-02-01",
          "South FC",
          { start: "2031-02-01", end: "2034-06-30", weekly_wage: 9000 },
          "free_agent_signing",
          "free_agent",
        ),
      ],
      TODAY,
    );

    expect(rows[1]).toMatchObject({ endedBy: "terminated" });
    expect(rows[1].replacedOn).toBeNull();
    expect(rows[0].inForce).toBe(true);
  });
});
