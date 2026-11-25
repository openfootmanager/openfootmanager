import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { InvokeArgs } from "@tauri-apps/api/core";

import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData, StaffData } from "../../store/gameStore";
import { createPlayer, createSeasonContext, createTeam } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

// The Contract tab's two previews (the wage bill and the squad-safety verdict) are the backend's
// answers about the whole club, so the open sheet asks again when anything they read changes, not
// only when this player does.

beforeAll(async () => {
  await i18nReady;
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});
vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => () => "stable",
}));
vi.mock("../../hooks/usePlayerKnowledge", () => ({
  usePlayerKnowledge: () => () => ({ kind: "own" }),
}));
vi.mock("../../hooks/useVisibleWage", () => ({
  useVisibleWage: () => ({ status: "answered", value: { kind: "exact", weekly: 12000 } }),
}));
vi.mock("../../hooks/usePotentialOpinion", () => ({
  usePotentialOpinion: () => ({ status: "answered", value: null }),
}));
vi.mock("../../hooks/usePositionRatings", () => ({
  usePositionRatings: () => ({ status: "answered", value: null }),
}));

const PROJECTION = {
  current_annual_wage_bill: 100_000,
  projected_annual_wage_bill: 100_000,
  annual_wage_budget: 200_000,
  annual_soft_cap: 220_000,
  current_weekly_wage_spend: 100_000,
  projected_weekly_wage_spend: 100_000,
  current_cash_runway_weeks: null,
  projected_cash_runway_weeks: null,
  currently_over_budget: false,
  policy_allows: true,
};

const PREVIEW = {
  player_id: "player-1",
  player_name: "Test Player",
  severance_cost: 50_000,
  squad_safety: {
    team_id: "team-1",
    projected_roster_size: 20,
    healthy_players: 19,
    healthy_goalkeepers: 2,
    effective_xi_size: 11,
    can_field_matchday_squad: true,
    missing_reasons: [],
  },
};

async function answer(command: string) {
  if (command === "preview_renewal_financial_impact") return { projection: PROJECTION };
  if (command === "preview_contract_termination") return { preview: PREVIEW };
  if (command === "get_player_match_history") return [];
  if (command === "get_player_stats_overview") return null;
  return null;
}

function gameWith(player: PlayerData, staff: StaffData[]): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: { current_date: "2026-08-01T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [createTeam({ id: "team-1", name: "Alpha FC" })],
    players: [player],
    staff,
    messages: [],
    news: [],
    league: { id: "league-1", name: "League", season: 1, fixtures: [], standings: [] },
    scouting_assignments: [],
    board_objectives: [],
  } as unknown as GameStateData;
}

const hire = { id: "coach-1", team_id: "team-1", wage: 4_000 } as unknown as StaffData;

function projectionAsks() {
  return vi
    .mocked(invoke)
    .mock.calls.filter(([command]) => command === "preview_renewal_financial_impact").length;
}

function terminationAsks() {
  return vi
    .mocked(invoke)
    .mock.calls.filter(([command]) => command === "preview_contract_termination").length;
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(answer);
});

describe("the Contract tab's previews follow the club, not only the player", () => {
  /**
   * Given his own player's Contract tab, open and answered
   * When the club hires a staff member (an agent over MCP, say) while the sheet is open, with the
   *      player, the date and his contract untouched
   * Then both previews are asked again: staff wages are in the club's wage bill
   */
  it("asks the previews again when the club hires staff while the sheet is open", async () => {
    const player = createPlayer({ id: "player-1", team_id: "team-1", wage: 12000 });
    const props = { isOwnClub: true, onGameUpdate: () => {}, onSelectTeam: () => {} };
    const { rerender } = render(
      <PlayerProfile player={player} gameState={gameWith(player, [])} {...props} />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Contract" }));
    await waitFor(() => {
      expect(projectionAsks()).toBe(1);
      expect(terminationAsks()).toBe(1);
    });

    rerender(<PlayerProfile player={player} gameState={gameWith(player, [hire])} {...props} />);

    await waitFor(() => {
      expect(projectionAsks()).toBe(2);
      expect(terminationAsks()).toBe(2);
    });
  });

  /**
   * Given the same open sheet
   * When the game state is replaced by an equal one (nothing the previews read changed)
   * Then neither preview is asked again
   */
  it("does not ask again when nothing the previews read has changed", async () => {
    const player = createPlayer({ id: "player-1", team_id: "team-1", wage: 12000 });
    const props = { isOwnClub: true, onGameUpdate: () => {}, onSelectTeam: () => {} };
    const { rerender } = render(
      <PlayerProfile player={player} gameState={gameWith(player, [hire])} {...props} />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Contract" }));
    await waitFor(() => expect(projectionAsks()).toBe(1));

    rerender(<PlayerProfile player={player} gameState={gameWith(player, [hire])} {...props} />);

    // Let any request a wrong key would start go out before looking.
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(projectionAsks()).toBe(1);
    expect(terminationAsks()).toBe(1);
  });
});

describe("the renewal dialog's budget preview", () => {
  /**
   * Given an offer whose preview exceeds the budget
   * When the manager changes the wage and optionally returns to the same amount
   * Then the old restriction stays cleared while the new request is pending
   */
  it.each([
    { scenario: "after lowering the wage", wages: [15_000] },
    { scenario: "after returning to the original wage", wages: [15_000, 60_000] },
  ])("clears the old budget warning $scenario", async ({ wages }) => {
    const pending = new Promise<never>(() => undefined);
    let highWageAnswered = false;
    vi.mocked(invoke).mockImplementation(async (command: string, payload?: InvokeArgs) => {
      if (command === "preview_renewal_financial_impact") {
        const wage = (payload as { weeklyWage?: number } | undefined)?.weeklyWage;
        if (wage === 60_000 && !highWageAnswered) {
          highWageAnswered = true;
          return { projection: { ...PROJECTION, policy_allows: false } };
        }
        if (wage === 15_000 || wage === 60_000) return pending;
      }
      return answer(command);
    });

    const player = createPlayer();
    render(
      <PlayerProfile
        player={player}
        gameState={gameWith(player, [])}
        isOwnClub
        onGameUpdate={() => {}}
        onSelectTeam={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Contract" }));
    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    const input = screen.getByLabelText("Offered Wage");
    const submit = screen.getByRole("button", { name: "Submit Offer" });
    const warning = "Offer exceeds the board wage pressure limit";
    fireEvent.change(input, { target: { value: "60000" } });
    expect(await screen.findByText(warning)).toBeInTheDocument();
    expect(submit).toBeDisabled();

    for (const wage of wages) {
      fireEvent.change(input, { target: { value: String(wage) } });
      expect(invoke).toHaveBeenLastCalledWith("preview_renewal_financial_impact", {
        playerId: player.id,
        weeklyWage: wage,
      });
    }

    expect(submit).toBeEnabled();
    expect(screen.queryByText(warning)).not.toBeInTheDocument();
    expect(screen.queryByText("Projected financial impact")).not.toBeInTheDocument();
  });
});
