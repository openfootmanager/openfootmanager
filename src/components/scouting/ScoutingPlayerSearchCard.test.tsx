import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData, TeamData } from "../../store/gameStore";
import ScoutingPlayerSearchCard from "./ScoutingPlayerSearchCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "scouting.findPlayers") return "Find Players";
      if (key === "scouting.searchPlaceholder") return "Search players";
      if (key === "scouting.player") return "Player";
      if (key === "scouting.pos") return "Pos";
      if (key === "scouting.age") return "Age";
      if (key === "scouting.team") return "Team";
      if (key === "scouting.value") return "Value";
      if (key === "scouting.action") return "Action";
      if (key === "scouting.scoutBtn") return "Scout";
      if (key === "scouting.previousPage") return "Previous page";
      if (key === "scouting.nextPage") return "Next page";
      if (key === "scouting.noPlayersFound") return "No players found";
      if (key === "scouting.noScoutsFree") return "No scouts free";
      if (key === "scouting.scoutingInProgress") {
        return "Scouting in progress";
      }
      if (key === "scouting.showingRange") {
        return `${params?.from}-${params?.to} of ${params?.total}`;
      }
      if (key === "common.all") return "All";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "common.viewTeam") return "View team";
      if (key === "squad.viewProfile") return "View profile";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-2",
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

describe("ScoutingPlayerSearchCard", () => {
  it("renders players and delegates search, filter, selection, scout, and pagination actions", () => {
    const onPositionFilterChange = vi.fn();
    const onSearchQueryChange = vi.fn();
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();
    const onSendScout = vi.fn();
    const onPreviousPage = vi.fn();
    const onNextPage = vi.fn();

    render(
      <ScoutingPlayerSearchCard
        players={[createPlayer()]}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", manager_id: "manager-2" }),
        ]}
        posFilter="All"
        searchQuery=""
        alreadyScoutingIds={new Set<string>()}
        availableScoutCount={1}
        sendingPlayerId={null}
        safePage={0}
        totalPages={2}
        totalPlayers={21}
        pageSize={20}
        onPositionFilterChange={onPositionFilterChange}
        onSearchQueryChange={onSearchQueryChange}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
        onSendScout={onSendScout}
        onPreviousPage={onPreviousPage}
        onNextPage={onNextPage}
      />,
    );

    expect(screen.getByText("Find Players")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "John Smith" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Def" }));
    expect(onPositionFilterChange).toHaveBeenCalledWith("Defender");

    fireEvent.change(screen.getByPlaceholderText("Search players"), {
      target: { value: "john" },
    });
    expect(onSearchQueryChange).toHaveBeenCalledWith("john");

    fireEvent.click(screen.getByRole("button", { name: "John Smith" }));
    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");

    fireEvent.click(screen.getByRole("button", { name: /^Scout$/i }));
    expect(onSendScout).toHaveBeenCalledWith("player-1");

    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    expect(onNextPage).toHaveBeenCalledOnce();

    const playerRow = screen.getByText("John Smith").closest("tr");
    expect(playerRow).not.toBeNull();

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("button", { name: "View team" }));
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("button", { name: "View profile" }));
    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
  });
});