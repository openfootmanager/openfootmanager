import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData, StaffData, TeamData } from "../../store/gameStore";
import YouthAcademyTab from "./YouthAcademyTab";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "youthAcademy.title") return "Youth Academy";
      if (key === "youthAcademy.playersUnder21") return `${params?.count} youth players`;
      if (key === "youthAcademy.recoveryTitle") return "Build your academy";
      if (key === "youthAcademy.recoveryDescription")
        return "Delegate eligible under-21 players from your senior squad or open scouting to look for more prospects.";
      if (key === "youthAcademy.eligibleSeniorPlayers")
        return `${params?.count} eligible senior players`;
      if (key === "youthAcademy.openScouting") return "Open scouting";
      if (key === "youthAcademy.noEligibleSeniorPlayers")
        return "No eligible under-21 senior players are available right now.";
      if (key === "youthAcademy.youthPlayers") return "Youth Players";
      if (key === "youthAcademy.avgOvr") return "Avg OVR";
      if (key === "youthAcademy.avgPotential") return "Avg Potential";
      if (key === "youthAcademy.highPotential") return "High Potential";
      if (key === "youthAcademy.youthCoach") return "Youth Coach";
      if (key === "youthAcademy.youthProspects") return "Youth Prospects";
      if (key === "youthAcademy.recruitmentWorkflowTitle") return "Youth recruitment workflow";
      if (key === "youthAcademy.recruitmentWorkflowHint")
        return "Start, cancel, or reassign academy searches without leaving the Youth Academy view.";
      if (key === "youthAcademy.noYouthPlayers") return "No youth players";
      if (key === "youthAcademy.delegateToYouthAcademy")
        return "Delegate to youth academy";
      if (key === "youthAcademy.promoteToSeniorSquad")
        return "Promote to senior squad";
      if (key === "youthAcademy.player") return "Player";
      if (key === "youthAcademy.pos") return "Pos";
      if (key === "youthAcademy.age") return "Age";
      if (key === "youthAcademy.ovr") return "OVR";
      if (key === "youthAcademy.potential") return "Potential";
      if (key === "youthAcademy.growth") return "Growth";
      if (key === "youthAcademy.traits") return "Traits";
      if (key === "youthAcademy.condition") return "Condition";
      if (key.startsWith("youthAcademy.pot")) return key.replace("youthAcademy.", "");
      if (key.startsWith("common.posAbbr.")) return key.replace("common.posAbbr.", "");
      if (key === "scouting.youthRecruitment") return "Youth Recruitment";
      if (key === "scouting.youthRecruitmentHint") return "Use a scout to search for academy prospects.";
      if (key === "scouting.startYouthSearch") return "Start youth search";
      if (key === "scouting.activeYouthSearches") return `${params?.count} active youth searches`;
      if (key === "scouting.noYouthSearches") return "No youth searches running";
      if (key === "scouting.noScoutsFree") return "No scouts free";
      if (key === "scouting.youthProspectSearch") return "Youth prospect search";
      if (key === "scouting.youthSearchScoutLabel") return "Scout";
      if (key === "scouting.youthSearchRegionLabel") return "Region";
      if (key === "scouting.youthSearchObjectiveLabel") return "Objective";
      if (key === "scouting.selectScout") return "Select scout";
      if (key === "scouting.regionDomestic") return "Domestic";
      if (key === "scouting.regionInternational") return "International";
      if (key === "scouting.objectiveBalanced") return "Balanced";
      if (key === "scouting.objectiveHighPotential") return "High potential";
      if (key === "scouting.objectiveReadySoon") return "Ready soon";
      if (key === "scouting.cancelSearch") return "Cancel";
      if (key === "scouting.reassignSearch") return "Reassign";
      if (key === "scouting.noAlternateScout") return "No alternate scout";
      if (key === "scouting.scoutLabel") return params?.name ? `Scout ${params.name}` : "Scout";
      if (key === "scouting.youthTargetLabel") return "Youth target";
      if (key === "scouting.youthAnyPosition") return "Any position";
      if (key === "scouting.daysLeft") return `${params?.days} days left`;
      if (key === "common.positions.Defender") return "Defender";
      if (key === "common.positions.Midfielder") return "Midfielder";
      if (key === "common.positions.Forward") return "Forward";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../TraitBadge", () => ({
  TraitList: () => <span>Traits</span>,
}));

const mockedInvoke = vi.mocked(invoke);

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
    date_of_birth: "2008-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 65,
      stamina: 65,
      strength: 65,
      agility: 65,
      passing: 65,
      shooting: 65,
      tackling: 40,
      dribbling: 65,
      defending: 40,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 50,
      teamwork: 60,
      leadership: 45,
      handling: 20,
      reflexes: 20,
      aerial: 55,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    retired: false,
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

function createGameState(players: PlayerData[]): GameStateData {
  return {
    clock: {
      current_date: "2026-08-10T00:00:00Z",
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
    players,
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

function createScout(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "staff-1",
    first_name: "Sam",
    last_name: "Scout",
    date_of_birth: "1985-01-01",
    nationality: "GB",
    role: "Scout",
    attributes: {
      coaching: 20,
      judging_ability: 65,
      judging_potential: 70,
      physiotherapy: 10,
    },
    team_id: "team-1",
    specialization: null,
    wage: 1000,
    contract_end: "2027-06-30",
    ...overrides,
  };
}

describe("YouthAcademyTab", () => {
  it("renders the empty state when the squad has no youth players", () => {
    render(
      <YouthAcademyTab
        gameState={createGameState([
          createPlayer({
            id: "player-young-senior",
            full_name: "Senior Prospect",
            date_of_birth: "2008-01-01",
          }),
        ])}
        onSelectPlayer={vi.fn()}
      />,
    );

    expect(screen.getByText("No youth players")).toBeInTheDocument();
    expect(screen.getByText("Build your academy")).toBeInTheDocument();
    expect(screen.getByText("1 eligible senior players")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Delegate to youth academy" }),
    ).toBeInTheDocument();
  });

  it("delegates eligible senior players from the recovery card", async () => {
    const gameState = createGameState([
      createPlayer({
        id: "player-young-senior",
        full_name: "Senior Prospect",
        date_of_birth: "2008-01-01",
      }),
    ]);
    const updatedGameState = {
      ...gameState,
      players: gameState.players.map((player) =>
        player.id === "player-young-senior"
          ? { ...player, squad_role: "Youth" as const }
          : player,
      ),
    };
    const onGameUpdate = vi.fn();
    mockedInvoke.mockResolvedValue(updatedGameState);

    render(
      <YouthAcademyTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Delegate to youth academy" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_player_squad_role", {
        playerId: "player-young-senior",
        squadRole: "Youth",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
    });
  });

  it("opens the scouting tab from the recovery card", () => {
    const onNavigate = vi.fn();

    render(
      <YouthAcademyTab
        gameState={createGameState([
          createPlayer({
            id: "player-young-senior",
            full_name: "Senior Prospect",
            date_of_birth: "2008-01-01",
          }),
        ])}
        onNavigate={onNavigate}
        onSelectPlayer={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Open scouting" }));

    expect(onNavigate).toHaveBeenCalledWith("Scouting");
  });

  it("starts youth recruitment directly from the youth academy view", async () => {
    const baseState = createGameState([
      createPlayer({
        id: "player-young",
        full_name: "Rising Star",
        date_of_birth: "2008-01-01",
        squad_role: "Youth",
      }),
    ]);
    const gameState = { ...baseState, staff: [createScout()] };
    const onGameUpdate = vi.fn();
    mockedInvoke.mockResolvedValue(gameState);

    render(
      <YouthAcademyTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "Youth target" }));
    fireEvent.click(screen.getByRole("option", { name: "Defender" }));
    fireEvent.click(screen.getByRole("button", { name: "Start youth search" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("start_youth_scouting", {
        scoutId: "staff-1",
        region: "Domestic",
        objective: "Balanced",
        targetPosition: "Defender",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(gameState);
    });
  });

  it("shows youth prospects only and routes row selection", () => {
    const onSelectPlayer = vi.fn();

    render(
      <YouthAcademyTab
        gameState={createGameState([
          createPlayer({
            id: "player-young",
            full_name: "Rising Star",
            date_of_birth: "2008-01-01",
            squad_role: "Youth",
          }),
          createPlayer({
            id: "player-older",
            full_name: "Senior Pro",
            date_of_birth: "1998-01-01",
          }),
        ])}
        onSelectPlayer={onSelectPlayer}
      />,
    );

    expect(screen.getByText("Rising Star")).toBeInTheDocument();
    expect(screen.queryByText("Senior Pro")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("Rising Star"));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-young");
  });

  it("promotes youth academy players through the context menu", async () => {
    const gameState = createGameState([
      createPlayer({
        id: "player-young",
        full_name: "Rising Star",
        date_of_birth: "2008-01-01",
        squad_role: "Youth",
      }),
    ]);
    const updatedGameState = {
      ...gameState,
      players: gameState.players.map((player) =>
        player.id === "player-young"
          ? { ...player, squad_role: "Senior" as const }
          : player,
      ),
    };
    const onGameUpdate = vi.fn();
    mockedInvoke.mockResolvedValue(updatedGameState);

    render(
      <YouthAcademyTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={vi.fn()}
      />,
    );

    const playerRow = screen.getByText("Rising Star").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(
      screen.getByRole("button", { name: "Promote to senior squad" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_player_squad_role", {
        playerId: "player-young",
        squadRole: "Senior",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
    });
  });
});