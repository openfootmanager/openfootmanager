import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { GameStateData } from "../store/gameStore";
import Dashboard from "./Dashboard";

const navigateMock = vi.fn();
const invokeMock = vi.fn();
const setGameStateMock = vi.fn();
const clearGameMock = vi.fn();
const markCleanMock = vi.fn();
const loadSettingsMock = vi.fn();

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

const gameState = createGameState();

vi.mock("react-router-dom", () => ({
  useNavigate: () => navigateMock,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onCloseRequested: vi.fn(() => Promise.resolve(() => {})),
    destroy: vi.fn(),
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const labels: Record<string, string> = {
        "dashboard.home": "Home",
        "dashboard.inbox": "Inbox",
        "dashboard.loading": "Loading",
        "dashboard.managers": "Managers",
        "dashboard.hallOfFame": "Hall of Fame",
        "continueMenu.goToField": "Go To Field",
        "continueMenu.goToFieldDesc": "desc",
        "continueMenu.watchSpectator": "Watch",
        "continueMenu.watchSpectatorDesc": "desc",
        "continueMenu.delegateAssistant": "Delegate",
        "continueMenu.delegateAssistantDesc": "desc",
      };

      return labels[key] ?? key;
    },
  }),
}));

vi.mock("../store/gameStore", () => ({
  useGameStore: () => ({
    hasActiveGame: true,
    managerName: "Jane Doe",
    gameState,
    setGameState: setGameStateMock,
    clearGame: clearGameMock,
    isDirty: false,
    markClean: markCleanMock,
  }),
}));

vi.mock("../store/settingsStore", () => ({
  useSettingsStore: () => ({
    settings: {
      language: "en",
      default_match_mode: "live",
    },
    loaded: true,
    loadSettings: loadSettingsMock,
  }),
}));

vi.mock("../hooks/useAdvanceTime", () => ({
  useAdvanceTime: () => ({
    isAdvancing: false,
    showContinueMenu: false,
    setShowContinueMenu: vi.fn(),
    showMatchConfirm: false,
    setShowMatchConfirm: vi.fn(),
    matchMode: "live",
    setMatchMode: vi.fn(),
    blockerModal: null,
    setBlockerModal: vi.fn(),
    handleContinue: vi.fn(),
    handleConfirmMatch: vi.fn(),
    handleSkipToMatchDay: vi.fn(),
  }),
}));

vi.mock("../components/dashboard/DashboardSidebar", () => ({
  default: ({ onNavClick, activeTab }: any) => (
    <div>
      <span>Sidebar {activeTab}</span>
      <button onClick={() => onNavClick("Inbox")}>nav-inbox</button>
      <button onClick={() => onNavClick("Managers")}>nav-managers</button>
    </div>
  ),
}));

vi.mock("../components/dashboard/DashboardHeader", () => ({
  default: ({ activeTabLabel, onBack, onSelectSearchPlayer, onSelectSearchTeam }: any) => (
    <div>
      <span>Header {activeTabLabel}</span>
      <button onClick={onBack}>header-back</button>
      <button onClick={() => onSelectSearchPlayer("player-1")}>search-player</button>
      <button onClick={() => onSelectSearchTeam("team-2")}>search-team</button>
    </div>
  ),
}));

vi.mock("../components/playerProfile/PlayerProfile", () => ({
  default: ({ onClose, onSelectTeam }: any) => (
    <div>
      <span>Player Profile Mock</span>
      <button onClick={onClose}>player-close</button>
      <button onClick={() => onSelectTeam("team-2")}>player-select-team</button>
    </div>
  ),
}));

vi.mock("../components/teamProfile", () => ({
  default: ({ onClose, onSelectPlayer }: any) => (
    <div>
      <span>Team Profile Mock</span>
      <button onClick={onClose}>team-close</button>
      <button onClick={() => onSelectPlayer("player-1")}>team-select-player</button>
    </div>
  ),
}));

vi.mock("../components/dashboard/DashboardAlerts", () => ({
  default: () => <div>Alerts Mock</div>,
}));

vi.mock("../components/dashboard/DashboardTabContent", () => ({
  default: ({ viewModel }: any) => <div>Tab Content {viewModel.activeTab}</div>,
}));

vi.mock("../components/dashboard/DashboardBlockerModal", () => ({
  default: () => null,
}));

vi.mock("../components/dashboard/DashboardCloseConfirmModal", () => ({
  default: () => null,
}));

vi.mock("../components/dashboard/DashboardExitConfirmModal", () => ({
  default: () => null,
}));

vi.mock("../components/dashboard/DashboardExitSavingModal", () => ({
  default: () => null,
}));

vi.mock("../components/dashboard/DashboardMatchConfirmModal", () => ({
  default: () => null,
}));

describe("Dashboard", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setGameStateMock.mockReset();
    clearGameMock.mockReset();
    markCleanMock.mockReset();
    loadSettingsMock.mockReset();
    navigateMock.mockReset();
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_active_game") {
        return gameState;
      }

      return null;
    });
  });

  it("supports search selection, profile switching, back-navigation, and tab switching", async () => {
    render(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText("Tab Content Home")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("search-player"));
    expect(screen.getByText("Player Profile Mock")).toBeInTheDocument();

    fireEvent.click(screen.getByText("player-select-team"));
    expect(screen.getByText("Team Profile Mock")).toBeInTheDocument();

    fireEvent.click(screen.getByText("header-back"));
    expect(screen.getByText("Player Profile Mock")).toBeInTheDocument();

    fireEvent.click(screen.getByText("header-back"));
    expect(screen.getByText("Tab Content Home")).toBeInTheDocument();

    fireEvent.click(screen.getByText("nav-inbox"));
    expect(screen.getByText("Tab Content Inbox")).toBeInTheDocument();

    fireEvent.click(screen.getByText("nav-managers"));
    expect(screen.getByText("Header Managers")).toBeInTheDocument();
  });
});