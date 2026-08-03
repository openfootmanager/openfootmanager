import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  SeasonAwardEntryData,
  SeasonManagerAwardEntryData,
} from "../../store/gameStore";
import TournamentsAwardCard from "./TournamentsAwardCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "common.viewTeam") return "View team";
      if (key === "squad.viewProfile") return "View profile";
      return key;
    },
  }),
}));

function playerEntry(
  overrides: Partial<SeasonAwardEntryData> = {},
): SeasonAwardEntryData {
  return {
    player_id: "player-1",
    player_name: "Ada Striker",
    team_id: "team-1",
    team_name: "Alpha FC",
    value: 24,
    ...overrides,
  };
}

function managerEntry(
  overrides: Partial<SeasonManagerAwardEntryData> = {},
): SeasonManagerAwardEntryData {
  return {
    manager_id: "manager-1",
    manager_name: "Bea Boss",
    team_id: "team-2",
    team_name: "Beta United",
    value: 30,
    win_rate: 72.4,
    ...overrides,
  };
}

function renderCard(
  props: Partial<React.ComponentProps<typeof TournamentsAwardCard>> = {},
) {
  return render(
    <TournamentsAwardCard
      icon={<span data-testid="icon" />}
      title="Golden Boot"
      subtitle="Most goals"
      entries={[playerEntry()]}
      unit="goals"
      emptyText="No data yet"
      onSelectTeam={vi.fn()}
      {...props}
    />,
  );
}

describe("TournamentsAwardCard", () => {
  it("shows the empty text instead of a list when there are no entries", () => {
    renderCard({ entries: [] });

    expect(screen.getByText("No data yet")).toBeInTheDocument();
    expect(
      screen.queryByTestId("tournaments-award-entry-player-1"),
    ).not.toBeInTheDocument();
  });

  it("ranks entries and shows each one's team and value", () => {
    renderCard({
      entries: [
        playerEntry(),
        playerEntry({
          player_id: "player-2",
          player_name: "Cy Winger",
          team_name: "Gamma City",
          value: 19,
        }),
      ],
    });

    expect(screen.getByText("Ada Striker")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("24")).toBeInTheDocument();
    expect(screen.getByText("Cy Winger")).toBeInTheDocument();
    expect(screen.getByText("19")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("rounds values by default and keeps two decimals when asked", () => {
    const { unmount } = renderCard({ entries: [playerEntry({ value: 8.6 })] });
    expect(screen.getByText("9")).toBeInTheDocument();
    unmount();

    renderCard({ entries: [playerEntry({ value: 8.6 })], decimal: true });
    expect(screen.getByText("8.60")).toBeInTheDocument();
  });

  it("reads a manager entry's win rate rather than its value", () => {
    renderCard({ entries: [managerEntry()], unit: "win rate" });

    expect(screen.getByText("Bea Boss")).toBeInTheDocument();
    expect(screen.getByText("72")).toBeInTheDocument();
  });

  it("offers a profile action for player entries", () => {
    const onSelectPlayer = vi.fn();
    renderCard({ onSelectPlayer });

    fireEvent.contextMenu(
      screen.getByTestId("tournaments-award-entry-player-1"),
    );
    fireEvent.click(screen.getByRole("menuitem", { name: "View profile" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
  });

  it("offers only the team action for manager entries, which have no profile", () => {
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();
    renderCard({ entries: [managerEntry()], onSelectPlayer, onSelectTeam });

    fireEvent.contextMenu(
      screen.getByTestId("tournaments-award-entry-manager-1"),
    );

    expect(
      screen.queryByRole("menuitem", { name: "View profile" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("menuitem", { name: "View team" }));
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
    expect(onSelectPlayer).not.toHaveBeenCalled();
  });
});
