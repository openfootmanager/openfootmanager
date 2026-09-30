import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PlayerMovementEntry } from "../../store/gameStore";
import PlayerProfileMovementHistoryCard from "./PlayerProfileMovementHistoryCard";

// A key-echo translator that also shows its parameters, so a test sees exactly which
// key and which values the card asked for.
const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

function renderCard(movementHistory: PlayerMovementEntry[]) {
  return render(<PlayerProfileMovementHistoryCard movementHistory={movementHistory} t={t} />);
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

describe("PlayerProfileMovementHistoryCard contract terms", () => {
  it("shows a contract's start, end and wage on its entry", () => {
    renderCard([renewal]);

    expect(screen.getByText("playerProfile.movementRenewal")).toBeInTheDocument();
    expect(
      screen.getByText(
        'playerProfile.movementContractRange {"start":"2026-01-10","end":"2029-06-30"}',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/playerProfile\.movementWage/)).toHaveTextContent("9,000");
  });

  it("shows an unknown start as an end alone, not as a blank or as today", () => {
    renderCard([
      {
        ...renewal,
        kind: "initial_contract",
        contract: { start: null, end: "2028-06-30", weekly_wage: 7000, source: "initial" },
      },
    ]);

    expect(
      screen.getByText('playerProfile.movementContractUntil {"end":"2028-06-30"}'),
    ).toBeInTheDocument();
    expect(screen.queryByText(/movementContractRange/)).not.toBeInTheDocument();
  });

  it("says when a contract was carried over from a save that predates the history", () => {
    renderCard([
      {
        date: "",
        kind: "initial_contract",
        to_team_id: "team-1",
        contract: { start: null, end: "2028-06-30", weekly_wage: 7000, source: "legacy_migrated" },
      },
    ]);

    expect(screen.getByText("playerProfile.movementLegacyContract")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.movementDateUnknown")).toBeInTheDocument();
  });

  it("names why a player was released", () => {
    renderCard([
      { date: "2026-06-30", kind: "released", from_team_id: "team-1", release_reason: "expired" },
      {
        date: "2026-03-01",
        kind: "released",
        from_team_id: "team-1",
        release_reason: "terminated",
      },
    ]);

    expect(screen.getByText("playerProfile.movementReleasedExpired")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.movementReleasedTerminated")).toBeInTheDocument();
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
});
