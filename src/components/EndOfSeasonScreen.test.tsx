import { fireEvent, render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";

import { useState } from "react";
import EndOfSeasonScreen, { type SeasonTransition } from "./EndOfSeasonScreen";
import type { GameStateData } from "../store/gameStore";
import { createAwards, createEndOfSeasonGame } from "../test-utils/endOfSeason";

const setShowFiredModal = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "endOfSeason.seasonComplete") return "Season complete";
      if (key === "endOfSeason.seasonLine") return `Season ${params?.season} ${params?.league}`;
      if (key === "endOfSeason.position") return "Position";
      if (key === "endOfSeason.points") return "Points";
      if (key === "endOfSeason.finalStandings") return "Final standings";
      if (key === "endOfSeason.processing") return "Processing";
      if (key === "endOfSeason.startNextSeason") return "Start next season";
      if (key === "endOfSeason.statsArchived") return "Stats archived";
      if (key === "endOfSeason.continueDashboard") return "Continue to dashboard";
      if (key === "tournaments.competitions.firstDivision")
        return `${params?.country} Erste Division`;
      if (key === "nations.de") return "Deutschland";
      if (key === "awardsCeremony.title") return "Awards Ceremony";
      if (key === "awardsCeremony.subtitle") return `Season ${params?.season} in ${params?.league}`;
      if (key === "awardsCeremony.managerOfSeason") return "Manager of the Season";
      if (key === "awardsCeremony.continue") return "Continue to dashboard";
      if (key === "awardsCeremony.goals") return "Goals";
      if (key === "awardsCeremony.rating") return "Rating";
      if (key === "awardsCeremony.winRate") return "Win rate";
      if (key === "common.won") return "W";
      if (key === "common.drawn") return "D";
      if (key === "common.lost") return "L";
      if (key === "common.gf") return "GF";
      if (key === "common.ga") return "GA";
      if (key === "common.place.1") return "1st";
      if (key === "common.place.2") return "2nd";
      if (key === "common.place.3") return "3rd";
      if (key === "common.place.other") return `${params?.n}th`;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../store/gameStore", async () => {
  const actual = await vi.importActual<typeof import("../store/gameStore")>("../store/gameStore");
  return {
    ...actual,
    useGameStore: Object.assign(
      (selector: (state: { setShowFiredModal: typeof setShowFiredModal }) => unknown) =>
        selector({ setShowFiredModal }),
      // The service layer reads the career token from the store's current game.
      { getState: () => ({ gameState: null }) },
    ),
  };
});

describe("EndOfSeasonScreen", () => {
  it("shows the awards ceremony after advancing to the next season", async () => {
    vi.mocked(invoke).mockResolvedValue({
      game: createEndOfSeasonGame(),
      summary: {
        season: 6,
        league_name: "Premier League",
        champion_id: "team-1",
        champion_name: "Alpha FC",
        user_position: 1,
        user_points: 83,
        user_won: 25,
        user_drawn: 8,
        user_lost: 5,
        user_goals_for: 72,
        user_goals_against: 30,
        golden_boot_player: "Victor Vale",
        golden_boot_goals: 24,
        poty_player: "Victor Vale",
        poty_rating: 7.8,
        total_teams: 2,
        season_awards: createAwards(),
      },
    });

    const onGameUpdate = vi.fn();

    // The dashboard holds the transition; this stands in for it.
    function Holder() {
      const [transition, setTransition] = useState<SeasonTransition | null>(null);
      return (
        <EndOfSeasonScreen
          gameState={createEndOfSeasonGame()}
          onGameUpdate={onGameUpdate as (game: GameStateData) => void}
          transition={transition}
          onTransitionChange={setTransition}
        />
      );
    }
    render(<Holder />);

    fireEvent.click(screen.getByRole("button", { name: "Start next season" }));

    expect(await screen.findByText("Awards Ceremony")).toBeInTheDocument();
    expect(screen.getAllByText("Manager of the Season").length).toBeGreaterThan(0);
    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
    expect(onGameUpdate).toHaveBeenCalled();
  });

  /** Given a summary for a keyed German division, when the ceremony opens, then its subtitle is localized. */
  it("shows the localized division in the ceremony subtitle", () => {
    const game = createEndOfSeasonGame();
    game.competitions = [
      {
        ...game.league!,
        id: "league-de",
        name: "Germany First Division",
        name_key: "tournaments.competitions.firstDivision",
        country_id: "DE",
      },
    ];
    const transition: SeasonTransition = {
      step: "ceremony",
      summary: {
        season: 6,
        league_name: "Germany First Division",
        competition_id: "league-de",
        champion_id: "team-1",
        champion_name: "Alpha FC",
        user_position: 1,
        user_points: 83,
        user_won: 25,
        user_drawn: 8,
        user_lost: 5,
        user_goals_for: 72,
        user_goals_against: 30,
        golden_boot_player: "Victor Vale",
        golden_boot_goals: 24,
        poty_player: "Victor Vale",
        poty_rating: 7.8,
        total_teams: 2,
        season_awards: createAwards(),
      },
    };
    render(
      <EndOfSeasonScreen
        gameState={game}
        onGameUpdate={vi.fn()}
        transition={transition}
        onTransitionChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Season 6 in Deutschland Erste Division")).toBeInTheDocument();
  });

  /**
   * Given a manager with no club (unemployed) when the season ends
   * When the review is drawn
   * Then there is no club summary at all: no "0th" place and no blank club name, and the league's
   *      final standings and champions still show.
   */
  it("shows the review without a club summary for an unemployed manager", () => {
    const game = createEndOfSeasonGame();
    game.manager.team_id = null;
    function Holder() {
      const [transition, setTransition] = useState<SeasonTransition | null>(null);
      return (
        <EndOfSeasonScreen
          gameState={game}
          onGameUpdate={vi.fn()}
          transition={transition}
          onTransitionChange={setTransition}
        />
      );
    }

    render(<Holder />);

    expect(screen.getByText("Season complete")).toBeInTheDocument();
    expect(screen.getByText("Final standings")).toBeInTheDocument();
    expect(screen.queryByText("0th")).not.toBeInTheDocument();
    expect(screen.queryByText("Position")).not.toBeInTheDocument();
  });
});
