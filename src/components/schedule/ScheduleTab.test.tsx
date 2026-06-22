import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { FixtureData, GameStateData, TeamData } from "../../store/gameStore";
import type { MatchdayGroup, ScheduleSlice } from "../../services/scheduleService";
import ScheduleTab from "./ScheduleTab";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../lib/seasonContext", () => ({
  resolveSeasonContext: () => ({
    phase: "RegularSeason",
    season_start: "2026-08-01",
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number> | string) => {
      if (key === "schedule.noLeague") return "No league";
      if (key === "schedule.fixtures") return "Fixtures";
      if (key === "schedule.standings") return "Standings";
      if (key === "schedule.international") return "International";
      if (key === "schedule.internationalDuty") return "On International Duty";
      if (key === "schedule.promotionZone") return "Promotion";
      if (key === "schedule.relegationZone") return "Relegation";
      if (key === "schedule.loadMore") return "Load more";
      if (key === "schedule.nextMatch") return "Next match";
      if (key === "schedule.noUpcoming") return "No upcoming fixtures.";
      if (key === "schedule.pastResults") return "Past results";
      if (key === "schedule.calendar.title") return "Calendar";
      if (key === "schedule.calendar.prevMonth") return "Previous month";
      if (key === "schedule.calendar.nextMonth") return "Next month";
      if (key === "schedule.season") return `Season ${(params as Record<string, number>)?.number}`;
      if (key === "schedule.matchday")
        return `Matchday ${(params as Record<string, number>)?.number}`;
      if (key === "common.team") return "Team";
      if (key === "common.viewTeam") return "View team";
      if (key === "common.played") return "P";
      if (key === "common.won") return "W";
      if (key === "common.drawn") return "D";
      if (key === "common.lost") return "L";
      if (key === "common.gf") return "GF";
      if (key === "common.ga") return "GA";
      if (key === "common.gd") return "GD";
      if (key === "common.pts") return "Pts";
      if (typeof params === "string") return params;
      if (params && typeof params === "object" && "defaultValue" in params)
        return params.defaultValue as string;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

const mockedInvoke = vi.mocked(invoke);

// ─── Helpers ────────────────────────────────────────────────────────────────

function makeTeam(overrides: Partial<TeamData> = {}): TeamData {
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

function makeFixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    matchday: 1,
    date: "2026-08-01",
    home_team_id: "team-1",
    away_team_id: "team-2",
    competition: "League",
    status: "Completed",
    result: { home_goals: 2, away_goals: 1, home_scorers: [], away_scorers: [] },
    ...overrides,
  };
}

function makeGroup(overrides: Partial<MatchdayGroup> = {}): MatchdayGroup {
  return {
    key: "league-1-league-1",
    date: "2026-09-05",
    matchday: 1,
    competition: "League",
    is_next_user_match: false,
    fixtures: [
      {
        id: "fix-1",
        matchday: 1,
        date: "2026-09-05",
        home_team_id: "team-1",
        home_team_name: "Alpha FC",
        away_team_id: "team-2",
        away_team_name: "Beta FC",
        competition: "League",
        competition_id: "league-1",
        status: "Scheduled",
        result: null,
      },
    ],
    ...overrides,
  };
}

function makeSlice(overrides: Partial<ScheduleSlice> = {}): ScheduleSlice {
  return {
    competition_id: "league-1",
    competition_name: "Premier League",
    today: "2026-08-15",
    past_groups: [],
    upcoming_groups: [makeGroup({ is_next_user_match: true })],
    next_user_match_date: "2026-09-05",
    ...overrides,
  };
}

function makeGameState(withCompetition: boolean): GameStateData {
  const competition: GameStateData["competitions"] = withCompetition
    ? [
        {
          id: "league-1",
          name: "Premier League",
          season: 1,
          participant_ids: ["team-1", "team-2"],
          fixtures: [makeFixture()],
          standings: [
            { team_id: "team-1", played: 1, won: 1, drawn: 0, lost: 0, goals_for: 2, goals_against: 1, points: 3 },
            { team_id: "team-2", played: 1, won: 0, drawn: 0, lost: 1, goals_for: 1, goals_against: 2, points: 0 },
          ],
        },
      ]
    : [];

  return {
    clock: { current_date: "2026-08-10T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: { matches_managed: 0, wins: 0, draws: 0, losses: 0, trophies: 0, best_finish: null },
      career_history: [],
    },
    teams: [
      makeTeam(),
      makeTeam({ id: "team-2", name: "Beta FC", short_name: "BET" }),
    ],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: withCompetition
      ? { id: "league-1", name: "Premier League", season: 1, fixtures: [makeFixture()], standings: [] }
      : null,
    competitions: competition,
    active_competition_ids: withCompetition ? ["league-1"] : [],
    scouting_assignments: [],
    board_objectives: [],
  };
}

// ─── Tests ──────────────────────────────────────────────────────────────────

describe("ScheduleTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    mockedInvoke.mockResolvedValue(makeSlice());
  });

  it("renders the empty state when there are no competitions", () => {
    render(<ScheduleTab gameState={makeGameState(false)} onSelectTeam={vi.fn()} />);
    expect(screen.getByText("No league")).toBeInTheDocument();
    // No invoke call if there are no competitions.
    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("fetches the schedule slice on mount and shows upcoming fixture", async () => {
    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByTestId("schedule-fixture-fix-1")).toBeInTheDocument();
    });
    expect(mockedInvoke).toHaveBeenCalledWith("get_schedule", {
      query: { competition_id: "league-1" },
    });
  });

  it("marks the next user match group with a badge", async () => {
    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByText("Next match")).toBeInTheDocument();
    });
  });

  it("fixtures view shows all fixture rows from the slice", async () => {
    const slice = makeSlice({
      upcoming_groups: [
        makeGroup({ key: "g1", fixtures: [{ id: "fix-upcoming", matchday: 1, date: "2026-09-05", home_team_id: "team-1", home_team_name: "Alpha FC", away_team_id: "team-2", away_team_name: "Beta FC", competition: "League", competition_id: "league-1", status: "Scheduled", result: null }] }),
      ],
      past_groups: [
        makeGroup({ key: "g0", date: "2026-08-01", fixtures: [{ id: "fix-past", matchday: 0, date: "2026-08-01", home_team_id: "team-2", home_team_name: "Beta FC", away_team_id: "team-1", away_team_name: "Alpha FC", competition: "League", competition_id: "league-1", status: "Completed", result: { home_goals: 1, away_goals: 2 } }] }),
      ],
    });
    mockedInvoke.mockResolvedValue(slice);

    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /Fixtures/i }));

    await waitFor(() => {
      expect(screen.getByTestId("schedule-fixture-fix-upcoming")).toBeInTheDocument();
      expect(screen.getByTestId("schedule-fixture-fix-past")).toBeInTheDocument();
    });
  });

  it("calendar view offers context menu actions for fixture teams", async () => {
    const onSelectTeam = vi.fn();
    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={onSelectTeam} />);

    await waitFor(() => {
      expect(screen.getByTestId("schedule-fixture-fix-1")).toBeInTheDocument();
    });

    fireEvent.contextMenu(screen.getByTestId("schedule-fixture-fix-1"));
    fireEvent.click(screen.getByRole("button", { name: /View team: Beta FC/i }));
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("switches to standings and lets the user click a team", async () => {
    const onSelectTeam = vi.fn();
    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={onSelectTeam} />);

    fireEvent.click(screen.getByRole("button", { name: /Standings/i }));
    await waitFor(() => {
      expect(screen.getByTestId("schedule-standings-row-team-2")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("Beta FC"));
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("marks promotion and relegation zones in standings", async () => {
    const state = makeGameState(true);
    const standing = (teamId: string, points: number) => ({
      team_id: teamId,
      played: 3,
      won: points / 3,
      drawn: 0,
      lost: 3 - points / 3,
      goals_for: points,
      goals_against: 3,
      points,
    });
    state.competitions = [
      {
        id: "eng-1",
        name: "First Division",
        season: 1,
        country_id: "ENG",
        priority: 0,
        participant_ids: ["team-1", "team-2", "team-3", "team-4"],
        fixtures: [],
        standings: [
          standing("team-1", 9),
          standing("team-2", 6),
          standing("team-3", 3),
          standing("team-4", 0),
        ],
      },
      {
        id: "eng-2",
        name: "Second Division",
        season: 1,
        country_id: "ENG",
        priority: 1,
        participant_ids: ["team-5", "team-6", "team-7", "team-8"],
        fixtures: [],
        standings: [],
      },
    ];
    state.active_competition_ids = ["eng-1", "eng-2"];

    render(<ScheduleTab gameState={state} onSelectTeam={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /Standings/i }));

    await waitFor(() => {
      expect(screen.getByTestId("standings-relegation-team-4")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("standings-relegation-team-3")).not.toBeInTheDocument();
    expect(screen.queryByTestId("standings-promotion-team-1")).not.toBeInTheDocument();
    expect(screen.getByText("Relegation")).toBeInTheDocument();
  });

  it("calendar shows past groups with load-more when there are many", async () => {
    const pastGroups: MatchdayGroup[] = Array.from({ length: 7 }, (_, i) => ({
      key: `past-${i}`,
      date: `2026-08-${String(i + 1).padStart(2, "0")}`,
      matchday: i + 1,
      competition: "League",
      is_next_user_match: false,
      fixtures: [
        {
          id: `fix-past-${i}`,
          matchday: i + 1,
          date: `2026-08-${String(i + 1).padStart(2, "0")}`,
          home_team_id: "team-1",
          home_team_name: "Alpha FC",
          away_team_id: "team-2",
          away_team_name: "Beta FC",
          competition: "League",
          competition_id: "league-1",
          status: "Completed",
          result: { home_goals: 1, away_goals: 0 },
        },
      ],
    }));
    mockedInvoke.mockResolvedValue(makeSlice({ upcoming_groups: [], past_groups: pastGroups }));

    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={vi.fn()} />);

    // First page of past groups (4) visible, more not visible yet.
    await waitFor(() => {
      expect(screen.getByTestId("schedule-fixture-fix-past-0")).toBeInTheDocument();
    });
    expect(screen.getByTestId("schedule-fixture-fix-past-3")).toBeInTheDocument();
    expect(screen.queryByTestId("schedule-fixture-fix-past-4")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Load more/i }));
    expect(screen.getByTestId("schedule-fixture-fix-past-4")).toBeInTheDocument();
    expect(screen.getByTestId("schedule-fixture-fix-past-6")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Load more/i })).not.toBeInTheDocument();
  });

  it("competition switcher triggers a new fetch", async () => {
    const state = makeGameState(true);
    state.competitions = [
      {
        id: "eng-1",
        name: "England First Division",
        season: 1,
        participant_ids: ["team-1", "team-2"],
        fixtures: [],
        standings: [],
      },
      {
        id: "bra-1",
        name: "Brazil First Division",
        season: 1,
        participant_ids: ["team-3", "team-4"],
        fixtures: [],
        standings: [],
      },
    ];
    state.active_competition_ids = ["eng-1", "bra-1"];

    render(<ScheduleTab gameState={state} onSelectTeam={vi.fn()} />);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_schedule", {
        query: { competition_id: "eng-1" },
      });
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("combobox"));
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: "Brazil First Division" }));
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_schedule", {
        query: { competition_id: "bra-1" },
      });
    });
  });

  it("hides the international toggle when there are no national-team fixtures", () => {
    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={vi.fn()} />);
    expect(
      screen.queryByRole("button", { name: /International/i }),
    ).not.toBeInTheDocument();
  });

  it("scrolls to the group card when a calendar day with fixtures is clicked", async () => {
    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={vi.fn()} />);

    // Wait for the slice to load so the group card div is in the DOM and its ref registered.
    await waitFor(() => {
      expect(screen.getByTestId("schedule-fixture-fix-1")).toBeInTheDocument();
    });

    // The calendar opens on 2026-09 (focusDate = next_user_match_date = "2026-09-05").
    // The group card registers its ref at group.date = "2026-09-05".
    fireEvent.click(screen.getByTestId("calendar-day-2026-09-05"));

    expect(Element.prototype.scrollIntoView).toHaveBeenCalledWith({
      behavior: "smooth",
      block: "start",
    });
  });

  it("auto-scrolls to next user fixture on load when mid-season past groups exist", async () => {
    const slice = makeSlice({
      past_groups: [
        makeGroup({
          key: "g-past",
          date: "2026-08-01",
          matchday: 0,
          fixtures: [
            {
              id: "fix-past",
              matchday: 0,
              date: "2026-08-01",
              home_team_id: "team-2",
              home_team_name: "Beta FC",
              away_team_id: "team-1",
              away_team_name: "Alpha FC",
              competition: "League",
              competition_id: "league-1",
              status: "Completed",
              result: { home_goals: 1, away_goals: 2 },
            },
          ],
        }),
      ],
      upcoming_groups: [makeGroup({ is_next_user_match: true })],
      next_user_match_date: "2026-09-05",
    });
    mockedInvoke.mockResolvedValue(slice);

    render(<ScheduleTab gameState={makeGameState(true)} onSelectTeam={vi.fn()} />);

    // Wait for upcoming fixture group to be in the DOM so its ref is registered.
    await waitFor(() => {
      expect(screen.getByTestId("schedule-fixture-fix-1")).toBeInTheDocument();
    });

    // The auto-scroll fires after a 50ms timeout; waitFor retries until scrollIntoView is called.
    await waitFor(() => {
      expect(Element.prototype.scrollIntoView).toHaveBeenCalledWith({
        behavior: "smooth",
        block: "start",
      });
    });
  });

  it("shows national-team fixtures and call-ups in the international view", async () => {
    const state = makeGameState(true);
    state.players = [
      { id: "p1", match_name: "Called Up", team_id: "team-1" } as GameStateData["players"][number],
    ];
    state.national_teams = [
      {
        id: "nt-eng",
        name: "England",
        football_nation: "EN",
        squad_player_ids: ["p1"],
        reputation: 80,
        fixtures: [
          makeFixture({
            id: "int-fix-1",
            competition: "InternationalNation",
            home_team_id: "nt-eng",
            away_team_id: "nt-esp",
            status: "Scheduled",
            result: null,
          }),
        ],
      },
    ] as GameStateData["national_teams"];

    render(<ScheduleTab gameState={state} onSelectTeam={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /International/i }));

    expect(screen.getByTestId("schedule-callup-p1")).toBeInTheDocument();
    expect(screen.getByTestId("schedule-international-int-fix-1")).toBeInTheDocument();
  });
});
