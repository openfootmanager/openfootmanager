import { render, screen, waitFor, within } from "@testing-library/react";
import { createInstance } from "i18next";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import en from "../../i18n/locales/en.json";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import { createPlayer, createSeasonContext, createTeam } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en } },
  interpolation: { escapeValue: false },
});
vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({ t: translations.t.bind(translations), i18n: translations }),
}));

// The other answers about the player have their own hooks and tests; here he is the club's own
// player on his wage, at no contract risk. The opinion of his potential goes through the real hook.
vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => () => "stable",
}));
vi.mock("../../hooks/usePlayerKnowledge", () => ({
  usePlayerKnowledge: () => () => ({ kind: "own" }),
}));
vi.mock("../../hooks/useVisibleWage", () => ({
  useVisibleWage: () => ({ kind: "exact", weekly: 12_000 }),
}));
vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const emptyStatsOverview = {
  percentileEligible: false,
  metrics: {
    shots: { total: 0, per90: null, percentile: null },
    shotsOnTarget: { total: 0, per90: null, percentile: null },
    passes: { completed: 0, attempted: 0, accuracy: null, percentile: null },
    tacklesWon: { total: 0, per90: null, percentile: null },
    interceptions: { total: 0, per90: null, percentile: null },
    foulsCommitted: { total: 0, per90: null, percentile: null },
  },
};

function gameWith(player: PlayerData): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: { current_date: "2026-09-15T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
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
    teams: [createTeam()],
    players: [player],
    staff: [],
    messages: [],
    news: [],
    league: { id: "league-1", name: "League", season: 1, fixtures: [], standings: [] },
    scouting_assignments: [],
    board_objectives: [],
  };
}

/** The backend answering the profile, with `opinion` as the club's opinion of his potential. */
function backendWith(opinion: PotentialOpinion | null) {
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === "get_potential_opinion") return opinion;
    if (command === "get_player_stats_overview") return emptyStatsOverview;
    if (command === "get_player_match_history") return [];
    return null;
  });
}

const renderProfile = () => {
  const player = createPlayer({ id: "player-1", team_id: "team-1" });
  render(<PlayerProfile player={player} gameState={gameWith(player)} isOwnClub />);
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

describe("the player profile's opinion of his potential", () => {
  /**
   * Given the club's own player, of whom the backend's coaches hold the opinion 76-84, band A
   * When his profile opens
   * Then it shows the coaches' opinion, the band's words and the range, asked of the backend for
   *      him
   */
  it("shows the coaches' opinion the backend holds of the player", async () => {
    backendWith({
      low: 76,
      high: 84,
      band: "A",
      certainty: "fairly_sure",
      source: { kind: "coaches" },
    });

    renderProfile();

    const opinion = within(await screen.findByRole("region", { name: "Coaches' opinion" }));
    expect(opinion.getByText("Could become a first-team regular")).toBeInTheDocument();
    expect(opinion.getByText("Potential 76–84")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("get_potential_opinion", { playerId: "player-1" });
  });

  /**
   * Given a player of whom the club holds no opinion
   * When his profile opens
   * Then nothing about his potential is shown
   */
  it("shows nothing of his potential when the club holds no opinion", async () => {
    backendWith(null);

    renderProfile();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("get_potential_opinion", { playerId: "player-1" }),
    );
    expect(screen.queryByRole("region", { name: "Coaches' opinion" })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Scout's opinion" })).not.toBeInTheDocument();
    expect(screen.queryByText(/^Potential /)).not.toBeInTheDocument();
  });

  /**
   * Given a player whose scout's report is from before the last season ended
   * When his profile opens
   * Then the scout's opinion shows, marked out of date
   */
  it("shows an old scout's opinion as out of date", async () => {
    backendWith({
      low: 58,
      high: 76,
      band: "Rest",
      certainty: "guess",
      source: { kind: "scout", scout_name: "Ann Scout", scouted_on: "2026-03-10", current: false },
    });

    renderProfile();

    const opinion = within(await screen.findByRole("region", { name: "Scout's opinion" }));
    expect(opinion.getByText("A guess")).toBeInTheDocument();
    expect(opinion.getByText("Out of date")).toBeInTheDocument();
  });
});
