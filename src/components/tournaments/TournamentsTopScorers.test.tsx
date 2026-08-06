import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsTopScorers from "./TournamentsTopScorers";
import type { TopScorerEntry } from "./TournamentsTab.helpers";

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");
  return {
    useTranslation: () => {
      useRef(null);
      return {
        t: (key: string) => {
          if (key === "common.viewTeam") return "View team";
          if (key === "squad.viewProfile") return "View profile";
          return key;
        },
      };
    },
  };
});

function scorer(overrides: Partial<TopScorerEntry> = {}): TopScorerEntry {
  return {
    playerId: "player-1",
    goals: 12,
    playerName: {
      match_name: "Striker",
      full_name: "Ada Striker",
      team_id: "team-1",
      team_name: "Alpha FC",
    },
    ...overrides,
  } as TopScorerEntry;
}

function renderScorers(
  props: Partial<React.ComponentProps<typeof TournamentsTopScorers>> = {},
) {
  return render(
    <TournamentsTopScorers
      topScorers={[scorer()]}
      onSelectTeam={vi.fn()}
      {...props}
    />,
  );
}

describe("TournamentsTopScorers", () => {
  it("says so when nobody has scored yet", () => {
    renderScorers({ topScorers: [] });

    expect(screen.getByText("tournaments.noGoals")).toBeInTheDocument();
  });

  it("ranks the scorers with their club and tally", () => {
    renderScorers({
      topScorers: [
        scorer(),
        scorer({
          playerId: "player-2",
          goals: 9,
          playerName: {
            match_name: "Winger",
            full_name: "Cy Winger",
            team_id: "team-2",
            team_name: "Beta United",
          },
        }),
      ],
    });

    expect(screen.getByText("Ada Striker")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("Cy Winger")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  // A player loaded from the slice can be clubless; the row still has to render.
  it("falls back to the team id, then to nothing, when there is no club name", () => {
    renderScorers({
      topScorers: [
        scorer({
          playerName: {
            match_name: "Striker",
            full_name: "Ada Striker",
            team_id: "team-9",
            team_name: null,
          },
        }),
      ],
    });

    expect(screen.getByText("team-9")).toBeInTheDocument();
  });

  it("offers the profile and the club when both are reachable", () => {
    const onSelectPlayer = vi.fn();
    renderScorers({ onSelectPlayer });

    fireEvent.contextMenu(screen.getByTestId("tournaments-top-scorer-player-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "View profile" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
  });

  it("offers only the club when there is no way to open a profile", () => {
    const onSelectTeam = vi.fn();
    renderScorers({ onSelectTeam });

    fireEvent.contextMenu(screen.getByTestId("tournaments-top-scorer-player-1"));

    expect(screen.queryByRole("menuitem", { name: "View profile" })).toBeNull();
    fireEvent.click(screen.getByRole("menuitem", { name: "View team" }));
    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
  });
});
