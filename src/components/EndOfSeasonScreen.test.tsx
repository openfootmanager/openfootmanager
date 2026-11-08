import { fireEvent, render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";

import { useState } from "react";
import EndOfSeasonScreen, { type SeasonTransition } from "./EndOfSeasonScreen";
import type { GameStateData } from "../store/gameStore";
import { createAwards, createEndOfSeasonGame } from "../test-utils/endOfSeason";

const setShowFiredModal = vi.fn();
const locale = vi.hoisted(() => ({ language: "en" }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (locale.language === "ru" && key.startsWith("common.ordinal.")) {
        return `${params?.n}-${key.includes(".place.") ? "е" : "й"}`;
      }
      key = key.replace("common.ordinal.place.", "common.ordinal.");
      if (key === "endOfSeason.seasonComplete") return "Season complete";
      if (key === "endOfSeason.seasonLine") return `Season ${params?.season} ${params?.league}`;
      if (key === "endOfSeason.position") return "Position";
      if (key === "endOfSeason.points") return "Points";
      if (key === "endOfSeason.finalStandings") return "Final standings";
      if (key === "endOfSeason.processing") return "Processing";
      if (key === "endOfSeason.startNextSeason") return "Start next season";
      if (key === "endOfSeason.statsArchived") return "Stats archived";
      if (key === "endOfSeason.continueDashboard") return "Continue to dashboard";
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
      // The language's ordinal forms (English here), picked by `formatOrdinal`.
      if (key === "common.ordinal.one") return `${params?.n}st`;
      if (key === "common.ordinal.two") return `${params?.n}nd`;
      if (key === "common.ordinal.few") return `${params?.n}rd`;
      if (key === "common.ordinal.other") return `${params?.n}th`;
      return key;
    },
    i18n: locale,
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

  /**
   * Given a club that finished 21st of 21 (an English ordinal ending that is not "th")
   * When the review is drawn
   * Then the place reads "21st", not "21th".
   */
  it("writes a 21st-place finish as 21st", () => {
    const game = createEndOfSeasonGame();
    const rival = game.league!.standings[1]!;
    game.league!.standings = [
      ...Array.from({ length: 20 }, (_, index) => ({
        ...rival,
        team_id: `rival-${index}`,
        points: 90 - index,
      })),
      { ...game.league!.standings[0]!, points: 1 },
    ];
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

    expect(screen.getByText("21st")).toBeInTheDocument();
    expect(screen.queryByText("21th")).not.toBeInTheDocument();
  });

  /** Given a second-place Russian club, when the review renders, then its place is neuter. */
  it("writes a Russian finishing place in the neuter form", () => {
    locale.language = "ru";
    try {
      const game = createEndOfSeasonGame();
      game.league!.standings[0]!.points = 1;
      game.league!.standings[1]!.points = 90;
      render(
        <EndOfSeasonScreen
          gameState={game}
          onGameUpdate={vi.fn()}
          transition={null}
          onTransitionChange={vi.fn()}
        />,
      );
      expect(screen.getByText("2-е")).toBeInTheDocument();
    } finally {
      locale.language = "en";
    }
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
