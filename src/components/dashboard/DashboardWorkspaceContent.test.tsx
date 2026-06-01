import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { GameStateData } from "../../store/gameStore";
import {
  createDashboardProfileNavigationState,
  selectDashboardPlayer,
  selectDashboardTeam,
} from "./dashboardProfileNavigation";
import { createDashboardTabContentModel } from "./dashboardTabContentModel";
import DashboardWorkspaceContent from "./DashboardWorkspaceContent";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("../playerProfile/PlayerProfile", () => ({
  default: ({
    onClose,
    onSelectTeam,
    startWithRenewalModal,
    startWithTerminationModal,
  }: any) => (
    <div>
      <span>Player Profile Mock</span>
      <span>{startWithRenewalModal ? "renewal-open" : "renewal-closed"}</span>
      <span>
        {startWithTerminationModal ? "termination-open" : "termination-closed"}
      </span>
      <button onClick={onClose}>close-player</button>
      <button onClick={() => onSelectTeam("team-2")}>select-team</button>
    </div>
  ),
}));

vi.mock("../teamProfile", () => ({
  default: ({ onClose, onSelectPlayer }: any) => (
    <div>
      <span>Team Profile Mock</span>
      <button onClick={onClose}>close-team</button>
      <button onClick={() => onSelectPlayer("player-2")}>select-player</button>
    </div>
  ),
}));

vi.mock("./DashboardAlerts", () => ({
  default: ({ onNavigate }: any) => (
    <button onClick={() => onNavigate("Inbox")}>alerts-mock</button>
  ),
}));

vi.mock("./DashboardTabContent", () => ({
  default: ({ viewModel }: any) => <div>Tab Content {viewModel.activeTab}</div>,
}));

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-07-10T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "England",
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
    teams: [
      {
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
      },
      {
        id: "team-2",
        name: "Beta FC",
        short_name: "BET",
        country: "GB",
        city: "Manchester",
        stadium_name: "Beta Ground",
        stadium_capacity: 28000,
        finance: 400000,
        manager_id: "manager-2",
        reputation: 48,
        wage_budget: 45000,
        transfer_budget: 200000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-3-3",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1901,
        colors: { primary: "#111111", secondary: "#eeeeee" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
    ],
    players: [
      {
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
        team_id: "team-1",
        retired: false,
        contract_end: "2026-10-15",
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
      },
    ],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("DashboardWorkspaceContent", () => {
  it("renders alerts and tab content when no profile is selected", () => {
    const gameState = createGameState();
    const onNavigate = vi.fn();

    render(
      <DashboardWorkspaceContent
        dashboardAlerts={[
          { id: "alert-1", text: "Alert", tab: "Inbox", severity: "info" },
        ]}
        gameState={gameState}
        profileNavigation={createDashboardProfileNavigationState("Home")}
        dashboardTabContentModel={createDashboardTabContentModel({
          activeTab: "Home",
          gameState,
          seasonComplete: false,
          visitedOnboardingTabs: new Set<string>(),
          initialMessageId: null,
          handlers: {
            onSelectPlayer: vi.fn(),
            onSelectTeam: vi.fn(),
            onGameUpdate: vi.fn(),
            onNavigate,
          },
        })}
        onBack={vi.fn()}
        onNavigate={onNavigate}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
        isUnemployed={false}
      />,
    );

    expect(screen.getByText("alerts-mock")).toBeInTheDocument();
    expect(screen.getByText("Tab Content Home")).toBeInTheDocument();

    fireEvent.click(screen.getByText("alerts-mock"));
    expect(onNavigate).toHaveBeenCalledWith("Inbox");
  });

  it("renders the player profile branch with renewal intent", () => {
    const gameState = createGameState();
    const profileNavigation = selectDashboardPlayer(
      createDashboardProfileNavigationState("Squad"),
      "player-1",
      { openRenewal: true },
    );

    render(
      <DashboardWorkspaceContent
        dashboardAlerts={[]}
        gameState={gameState}
        profileNavigation={profileNavigation}
        dashboardTabContentModel={createDashboardTabContentModel({
          activeTab: "Squad",
          gameState,
          seasonComplete: false,
          visitedOnboardingTabs: new Set<string>(),
          initialMessageId: null,
          handlers: {
            onSelectPlayer: vi.fn(),
            onSelectTeam: vi.fn(),
            onGameUpdate: vi.fn(),
            onNavigate: vi.fn(),
          },
        })}
        onBack={vi.fn()}
        onNavigate={vi.fn()}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
        isUnemployed={false}
      />,
    );

    expect(screen.getByText("Player Profile Mock")).toBeInTheDocument();
    expect(screen.getByText("renewal-open")).toBeInTheDocument();
    expect(screen.getByText("termination-closed")).toBeInTheDocument();
    expect(screen.queryByText("Tab Content Squad")).not.toBeInTheDocument();
  });

  it("passes the termination intent to the player profile branch", () => {
    const gameState = createGameState();
    const profileNavigation = selectDashboardPlayer(
      createDashboardProfileNavigationState("Squad"),
      "player-1",
      { openTermination: true },
    );

    render(
      <DashboardWorkspaceContent
        dashboardAlerts={[]}
        gameState={gameState}
        profileNavigation={profileNavigation}
        dashboardTabContentModel={createDashboardTabContentModel({
          activeTab: "Squad",
          gameState,
          seasonComplete: false,
          visitedOnboardingTabs: new Set<string>(),
          initialMessageId: null,
          handlers: {
            onSelectPlayer: vi.fn(),
            onSelectTeam: vi.fn(),
            onGameUpdate: vi.fn(),
            onNavigate: vi.fn(),
          },
        })}
        onBack={vi.fn()}
        onNavigate={vi.fn()}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
        isUnemployed={false}
      />,
    );

    expect(screen.getByText("termination-open")).toBeInTheDocument();
    expect(screen.getByText("renewal-closed")).toBeInTheDocument();
  });

  it("renders the team profile branch when a team is selected", () => {
    const gameState = createGameState();
    const profileNavigation = selectDashboardTeam(
      createDashboardProfileNavigationState("Teams"),
      "team-2",
    );

    render(
      <DashboardWorkspaceContent
        dashboardAlerts={[]}
        gameState={gameState}
        profileNavigation={profileNavigation}
        dashboardTabContentModel={createDashboardTabContentModel({
          activeTab: "Teams",
          gameState,
          seasonComplete: false,
          visitedOnboardingTabs: new Set<string>(),
          initialMessageId: null,
          handlers: {
            onSelectPlayer: vi.fn(),
            onSelectTeam: vi.fn(),
            onGameUpdate: vi.fn(),
            onNavigate: vi.fn(),
          },
        })}
        onBack={vi.fn()}
        onNavigate={vi.fn()}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
        isUnemployed={false}
      />,
    );

    expect(screen.getByText("Team Profile Mock")).toBeInTheDocument();
    expect(screen.queryByText("Tab Content Teams")).not.toBeInTheDocument();
  });
});