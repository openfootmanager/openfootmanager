import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PlayerMovementEntry } from "../../store/gameStore";
import PlayerProfileMovementHistoryCard from "./PlayerProfileMovementHistoryCard";

// A key-echo translator that also shows its parameters, so a test sees exactly which
// key and which values the card asked for.
const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

function renderCard(movementHistory: PlayerMovementEntry[]) {
  return render(
    <PlayerProfileMovementHistoryCard movementHistory={movementHistory} language="en" t={t} />,
  );
}

const renewal: PlayerMovementEntry = {
  date: "2026-01-10",
  kind: "renewal",
  to_team_id: "team-1",
  to_team_name: "Alpha FC",
  contract: {
    start: "2026-01-10",
    end: "2029-06-30",
    weekly_wage: 9000,
    source: "renewal",
  },
};

describe("PlayerProfileMovementHistoryCard moves", () => {
  it("shows an entry with no date as an unknown date, not as today", () => {
    renderCard([{ date: "", kind: "initial_contract", to_team_id: "team-1" }]);

    expect(screen.getByText("playerProfile.movementDateUnknown")).toBeInTheDocument();
  });

  it("still renders an entry that carries no contract, exactly as before", () => {
    renderCard([
      {
        date: "2025-08-01",
        kind: "loan_start",
        from_team_name: "Beta FC",
        to_team_name: "Alpha FC",
        loan_end_date: "2026-05-31",
      },
    ]);

    expect(screen.getByText("playerProfile.movementLoanStart")).toBeInTheDocument();
    expect(screen.queryByText(/movementContract/)).not.toBeInTheDocument();
    expect(screen.queryByText(/movementWage/)).not.toBeInTheDocument();
  });

  it("lists the newest first, and within one day the later entry first", () => {
    renderCard([
      { date: "2026-06-30", kind: "released", from_team_id: "team-1", release_reason: "expired" },
      {
        date: "2026-06-30",
        kind: "free_agent_signing",
        to_team_id: "team-2",
        contract: {
          start: "2026-06-30",
          end: "2029-06-30",
          weekly_wage: 1500,
          source: "free_agent",
        },
      },
      { date: "2024-07-01", kind: "initial_contract", to_team_id: "team-1" },
    ]);

    const badges = screen
      .getAllByText(/^playerProfile\.movement(Released|FreeAgentSigning|InitialContract)$/)
      .map((node) => node.textContent);
    expect(badges).toEqual([
      "playerProfile.movementFreeAgentSigning",
      "playerProfile.movementReleased",
      "playerProfile.movementInitialContract",
    ]);
  });

  it("says a renewal was with a club, not that the player joined it", () => {
    renderCard([renewal]);

    expect(
      screen.getByText('playerProfile.movementRenewedWith {"club":"Alpha FC"}'),
    ).toBeInTheDocument();
    expect(screen.queryByText(/playerProfile\.movementTo/)).not.toBeInTheDocument();
  });

  it("says an opening contract was at a club, not that the player joined it", () => {
    renderCard([
      {
        date: "2026-07-01",
        kind: "initial_contract",
        to_team_id: "team-1",
        to_team_name: "Alpha FC",
        contract: { start: null, end: "2028-06-30", weekly_wage: 7000, source: "initial" },
      },
    ]);

    expect(
      screen.getByText('playerProfile.movementAtClub {"club":"Alpha FC"}'),
    ).toBeInTheDocument();
    expect(screen.queryByText(/playerProfile\.movementTo/)).not.toBeInTheDocument();
  });

  it("still says a signing or a transfer joined the club", () => {
    renderCard([
      {
        date: "2026-07-01",
        kind: "free_agent_signing",
        to_team_id: "team-1",
        to_team_name: "Alpha FC",
      },
    ]);

    expect(screen.getByText('playerProfile.movementTo {"to":"Alpha FC"}')).toBeInTheDocument();
  });
});

describe("PlayerProfileMovementHistoryCard moves, not contracts", () => {
  /**
   * Given a renewal, a legacy contract and a release, whose contracts the Contract tab now lists
   * When the movement history renders
   * Then each entry says what moved and when (and the fee and loan end), and none repeats a
   *      contract's span, wage, legacy label or release reason: nothing shows twice
   */
  it("leaves the contract terms and the release reason to the Contract tab", () => {
    renderCard([
      renewal,
      {
        date: "",
        kind: "initial_contract",
        to_team_id: "team-1",
        to_team_name: "Alpha FC",
        contract: { start: null, end: "2028-06-30", weekly_wage: 7000, source: "legacy_migrated" },
      },
      {
        date: "2027-06-30",
        kind: "released",
        from_team_id: "team-1",
        from_team_name: "Alpha FC",
        release_reason: "expired",
      },
    ]);

    expect(screen.getByText("playerProfile.movementRenewal")).toBeInTheDocument();
    expect(screen.queryByText(/playerProfile\.movementContract/)).toBeNull();
    expect(screen.queryByText(/playerProfile\.movementWage/)).toBeNull();
    expect(screen.queryByText("playerProfile.movementLegacyContract")).toBeNull();
    expect(screen.queryByText("playerProfile.movementReleasedExpired")).toBeNull();
  });
});

describe("PlayerProfileMovementHistoryCard dates", () => {
  /**
   * Given a loan that began on 1 August 2025 and ends on 31 May 2026
   * When the movement history renders in English
   * Then the date of the move and the end of the loan are written as the language writes dates,
   *      the same way the Contract tab writes its own, not as ISO days
   */
  it("writes a move's date and a loan's end the way the language writes dates", () => {
    renderCard([
      {
        date: "2025-08-01",
        kind: "loan_start",
        from_team_name: "Beta FC",
        to_team_name: "Alpha FC",
        loan_end_date: "2026-05-31",
      },
    ]);

    expect(screen.getByText("Aug 1, 2025")).toBeInTheDocument();
    expect(
      screen.getByText('playerProfile.movementLoanUntil {"date":"May 31, 2026"}'),
    ).toBeInTheDocument();
  });
});
