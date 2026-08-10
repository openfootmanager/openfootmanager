import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsAwardsGrid from "./TournamentsAwardsGrid";
import type { SeasonAwardsData } from "../../store/gameStore";

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");
  return {
    useTranslation: () => {
      // The real useTranslation always costs a hook slot; a hook-free mock lets
      // a component that returns early render zero hooks, which hides
      // hook-order bugs entirely. See FinancesTab.test.tsx.
      useRef(null);
      return { t: (key: string) => key };
    },
  };
});

function awardsData(): SeasonAwardsData {
  const playerEntries = (id: string) => [
    {
      player_id: id,
      player_name: `Player ${id}`,
      team_id: "team-1",
      team_name: "Alpha FC",
      value: 12.5,
    },
  ];

  return {
    manager_of_season: [
      {
        manager_id: "manager-1",
        manager_name: "Bea Boss",
        team_id: "team-2",
        team_name: "Beta United",
        value: 30,
        win_rate: 72.4,
      },
    ],
    golden_boot: playerEntries("gb"),
    assist_king: playerEntries("ak"),
    player_of_year: playerEntries("poy"),
    clean_sheet_king: playerEntries("csk"),
    most_appearances: playerEntries("ma"),
    young_player: playerEntries("yp"),
  };
}

function renderGrid(
  props: Partial<React.ComponentProps<typeof TournamentsAwardsGrid>> = {},
) {
  return render(
    <TournamentsAwardsGrid
      awards={awardsData()}
      awardsLoadState="idle"
      retryAwards={vi.fn()}
      seasonComplete
      onSelectTeam={vi.fn()}
      {...props}
    />,
  );
}

describe("TournamentsAwardsGrid", () => {
  it("shows every award once the season's data is in", () => {
    renderGrid();

    for (const key of [
      "tournaments.awards.managerOfSeasonTitle",
      "tournaments.awards.goldenBootTitle",
      "tournaments.awards.assistKingTitle",
      "tournaments.awards.playerOfYearTitle",
      "tournaments.awards.goldenGloveTitle",
      "tournaments.awards.everPresentTitle",
      "tournaments.awards.youngPlayerTitle",
    ]) {
      expect(screen.getByText(key)).toBeInTheDocument();
    }
  });

  // Rating awards read to two decimals; tallies round. Getting these the wrong
  // way round is the mistake a table of cards makes easy to introduce.
  it("keeps two decimals for ratings and rounds plain tallies", () => {
    renderGrid();

    // player_of_year and young_player are the decimal awards, on 12.5.
    expect(screen.getAllByText("12.50")).toHaveLength(2);
    // golden_boot, assist_king, clean_sheet_king, most_appearances round it.
    expect(screen.getAllByText("13")).toHaveLength(4);
  });

  it("warns that the table is provisional until the season ends", () => {
    renderGrid({ seasonComplete: false });

    expect(
      screen.getByText("tournaments.awards.currentLeaders"),
    ).toBeInTheDocument();
  });

  it("does not warn once the season is complete", () => {
    renderGrid();

    expect(
      screen.queryByText("tournaments.awards.currentLeaders"),
    ).not.toBeInTheDocument();
  });

  it("offers a retry when the awards failed to load", () => {
    const retryAwards = vi.fn();
    renderGrid({ awards: null, awardsLoadState: "error", retryAwards });

    fireEvent.click(screen.getByRole("button", { name: "common.retry" }));

    expect(retryAwards).toHaveBeenCalledTimes(1);
  });

  // "No data yet" describes an empty season, not a request that failed — and
  // it read as a contradiction next to a retry button.
  it("says the awards failed rather than that there are none", () => {
    renderGrid({ awards: null, awardsLoadState: "error" });

    expect(
      screen.getByText("tournaments.awards.loadFailed"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("tournaments.awards.noDataYet"),
    ).not.toBeInTheDocument();
  });

  it("shows a loading note while the awards are on their way", () => {
    renderGrid({ awards: null, awardsLoadState: "loading" });

    expect(screen.getByText("tournaments.loadingAwards")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "common.retry" }),
    ).not.toBeInTheDocument();
  });
});
