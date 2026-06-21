import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../../store/gameStore";
import type {
  TeamCard,
  TeamsDirectory,
  TeamsDirectoryQuery,
} from "../../services/teamsService";
import TeamsListTab from "./TeamsListTab";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | { defaultValue?: string }) => {
      const labels: Record<string, string> = {
        "teams.yourTeam": "Your Team",
        "common.position": "Position",
        "teams.squad": "Squad",
        "teams.avgOvr": "Avg OVR",
        "teams.rep": "Rep",
        "common.value": "Value",
        "common.pts": "Pts",
        "teams.est": "Est",
        "teams.searchPlaceholder": "Search clubs",
        "teams.noResults": "No clubs match your search.",
        "teams.otherClubs": "Other clubs",
        "common.playStyles.Balanced": "Equilibrado",
        "common.playStyles.Counter": "Contra-ataque",
      };
      if (labels[key]) return labels[key];
      if (fallback && typeof fallback === "object") {
        return fallback.defaultValue ?? key;
      }
      return fallback ?? key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../ui", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../ui")>();
  return {
    ...actual,
    TeamLocation: ({ city, countryCode }: { city: string; countryCode: string }) => (
      <span>{`${city}, ${countryCode}`}</span>
    ),
  };
});

const mockedInvoke = vi.mocked(invoke);

function gameStateWithManagerTeam(teamId: string | null): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: teamId,
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
    teams: [],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

function buildCard(overrides: {
  id: string;
  name: string;
  city?: string;
  short_name?: string;
  founded_year?: number;
  play_style?: string;
  league_pos: number;
  points?: number;
}): TeamCard {
  return {
    team: {
      id: overrides.id,
      name: overrides.name,
      short_name: overrides.short_name ?? overrides.name.slice(0, 3).toUpperCase(),
      city: overrides.city ?? "London",
      country: "GB",
      colors: { primary: "#000000", secondary: "#ffffff" },
      formation: "4-4-2",
      play_style: overrides.play_style ?? "Balanced",
      founded_year: overrides.founded_year ?? 1900,
      reputation: 50,
      media: { logo: null },
    },
    roster_size: 22,
    avg_ovr: 65,
    total_value: 1_000_000,
    league_pos: overrides.league_pos,
    standing:
      overrides.points !== undefined
        ? {
            played: 1,
            won: 0,
            drawn: 0,
            lost: 0,
            goals_for: 0,
            goals_against: 0,
            points: overrides.points,
          }
        : null,
  };
}

function applySearch(cards: TeamCard[], search: string | null): TeamCard[] {
  if (!search) return cards;
  const needle = search.toLowerCase();
  return cards.filter(
    (card) =>
      card.team.name.toLowerCase().includes(needle) ||
      card.team.city.toLowerCase().includes(needle),
  );
}

function buildDirectory(cards: TeamCard[], search: string | null): TeamsDirectory {
  const filtered = applySearch(cards, search);
  if (filtered.length === 0) {
    return { regions: [] };
  }
  const sorted = [...filtered].sort((a, b) => a.league_pos - b.league_pos);
  return {
    regions: [
      {
        id: "europe",
        team_count: sorted.length,
        leagues: [
          {
            id: "league-1",
            name: "League",
            teams: sorted,
          },
        ],
      },
    ],
  };
}

function setupDirectoryMock(cards: TeamCard[]) {
  mockedInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
    if (cmd !== "get_teams_directory") return undefined;
    const query = (args as { query: TeamsDirectoryQuery }).query;
    return buildDirectory(cards, query.search);
  });
}

describe("TeamsListTab", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("orders teams by league position and marks the user team", async () => {
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 2, points: 1 }),
      buildCard({ id: "team-2", name: "Beta FC", league_pos: 1, points: 3 }),
    ]);

    render(
      <TeamsListTab
        gameState={gameStateWithManagerTeam("team-1")}
        onSelectTeam={vi.fn()}
      />,
    );

    await screen.findByText("Beta FC");
    const headings = screen.getAllByRole("heading", { level: 3 });
    expect(headings[0]).toHaveTextContent("Beta FC");
    expect(headings[1]).toHaveTextContent("Alpha FC");
    expect(screen.getByText("Your Team")).toBeInTheDocument();
  });

  it("filters clubs by name via the search box", async () => {
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 2 }),
      buildCard({ id: "team-2", name: "Beta FC", league_pos: 1 }),
    ]);

    render(
      <TeamsListTab
        gameState={gameStateWithManagerTeam("team-1")}
        onSelectTeam={vi.fn()}
      />,
    );

    await screen.findByText("Alpha FC");

    fireEvent.change(screen.getByPlaceholderText("Search clubs"), {
      target: { value: "beta" },
    });

    await waitFor(() => {
      expect(screen.queryByText("Alpha FC")).not.toBeInTheDocument();
    });
    expect(screen.getByText("Beta FC")).toBeInTheDocument();
  });

  it("shows an empty state when no clubs match", async () => {
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 1 }),
    ]);

    render(
      <TeamsListTab
        gameState={gameStateWithManagerTeam("team-1")}
        onSelectTeam={vi.fn()}
      />,
    );

    await screen.findByText("Alpha FC");

    fireEvent.change(screen.getByPlaceholderText("Search clubs"), {
      target: { value: "zzzzz" },
    });

    await waitFor(() => {
      expect(
        screen.getByText("No clubs match your search."),
      ).toBeInTheDocument();
    });
  });

  it("selects a team when its card is clicked", async () => {
    const onSelectTeam = vi.fn();
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 2 }),
      buildCard({ id: "team-2", name: "Beta FC", league_pos: 1 }),
    ]);

    render(
      <TeamsListTab
        gameState={gameStateWithManagerTeam("team-1")}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(await screen.findByText("Beta FC"));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("renders translated play styles instead of raw values", async () => {
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 1, play_style: "Balanced" }),
      buildCard({ id: "team-2", name: "Beta FC", league_pos: 2, play_style: "Counter" }),
    ]);

    render(
      <TeamsListTab
        gameState={gameStateWithManagerTeam("team-1")}
        onSelectTeam={vi.fn()}
      />,
    );

    expect(await screen.findByText(/4-4-2 — Equilibrado/)).toBeInTheDocument();
    expect(screen.getByText(/4-4-2 — Contra-ataque/)).toBeInTheDocument();
    expect(screen.queryByText(/4-4-2 — Balanced/)).not.toBeInTheDocument();
    expect(screen.queryByText(/4-4-2 — Counter/)).not.toBeInTheDocument();
  });
});
