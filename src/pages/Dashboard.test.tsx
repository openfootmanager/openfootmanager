import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { GameStateData } from "../store/gameStore";
import type { LeagueData, SeasonContextData } from "../store/types";
import { applyExtraTranslations } from "../lib/extraTranslations";
import Dashboard from "./Dashboard";
import i18n, { i18nReady, SUPPORTED_LANGUAGES } from "../i18n";
import { useTranslation } from "react-i18next";

const { listenMock, registeredEventHandlers, matchConfirmState } = vi.hoisted(() => {
  const handlers = new Map<string, (...args: unknown[]) => unknown>();

  return {
    listenMock: vi.fn((event: string, handler: (...args: unknown[]) => unknown) => {
      handlers.set(event, handler);
      return Promise.resolve(vi.fn());
    }),
    registeredEventHandlers: handlers,
    matchConfirmState: { visible: false },
  };
});

const navigateMock = vi.fn();
const invokeMock = vi.fn();
const setGameStateMock = vi.fn();
const clearGameMock = vi.fn();
const markCleanMock = vi.fn();
const loadSettingsMock = vi.fn();
const destroyMock = vi.fn();
let closeRequested: ((event: { preventDefault: () => void }) => Promise<void>) | undefined;
let isDirty = false;

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
    extra_translations: {
      en: { tournaments: { customCup: "Custom Cup" } },
    },
  };
}

let gameState = createGameState();
function createBackendFinanceSnapshot() {
  return {
    annual_wage_bill: 12000,
    weekly_wage_spend: 12000,
    weekly_wage_budget: 50000,
    weekly_recurring_income: 12000,
    weekly_sponsor_income: 12000,
    projected_weekly_net: 0,
    cash_runway_weeks: null as number | null,
    wage_budget_usage_percent: 24,
    currently_in_debt: false,
    currently_over_budget: false,
    wage_budget_status: "stable",
    runway_status: "stable",
    overall_status: "stable",
    marketing_campaign_cooldown_days_remaining: 0,
  };
}
let backendFinanceSnapshot = createBackendFinanceSnapshot();

vi.mock("../lib/extraTranslations", () => ({
  applyExtraTranslations: vi.fn(),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: () => navigateMock,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onCloseRequested: vi.fn((handler: (event: { preventDefault: () => void }) => Promise<void>) => {
      closeRequested = handler;
      return Promise.resolve(() => {});
    }),
    destroy: destroyMock,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("../store/gameStore", () => ({
  useGameStore: () => ({
    hasActiveGame: true,
    managerName: "Jane Doe",
    gameState,
    setGameState: setGameStateMock,
    clearGame: clearGameMock,
    isDirty,
    markClean: markCleanMock,
  }),
}));

vi.mock("../store/settingsStore", () => {
  const getState = () => ({
    settings: {
      language: "en",
      default_match_mode: "live",
    },
    currency: { code: "EUR", symbol: "€", exchange_rate: 1 },
    supportedCurrencies: { EUR: { code: "EUR", symbol: "€", exchange_rate: 1 } },
    loaded: true,
  });
  const useSettingsStore = Object.assign(
    () => ({ ...getState(), loadSettings: loadSettingsMock }),
    { getState },
  );
  return { useSettingsStore };
});

vi.mock("../hooks/useAdvanceTime", () => ({
  useAdvanceTime: () => ({
    isAdvancing: false,
    showContinueMenu: false,
    setShowContinueMenu: vi.fn(),
    showMatchConfirm: matchConfirmState.visible,
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
  default: ({
    onNavClick,
    activeTab,
    onExitClick,
  }: {
    onExitClick: () => void;
    onNavClick: (tab: string) => void;
    activeTab: string;
  }) => (
    <div>
      <span>Sidebar {activeTab}</span>
      <button type="button" onClick={onExitClick}>
        exit-menu
      </button>
      <button type="button" onClick={() => onNavClick("Inbox")}>
        nav-inbox
      </button>
      <button type="button" onClick={() => onNavClick("Managers")}>
        nav-managers
      </button>
    </div>
  ),
}));

vi.mock("../components/dashboard/DashboardHeader", () => ({
  default: ({
    activeTabLabel,
    onBack,
    onSelectSearchPlayer,
    onSelectSearchTeam,
    onSave,
  }: {
    onSave: () => void;
    activeTabLabel: string;
    onBack: () => void;
    onSelectSearchPlayer: (playerId: string) => void;
    onSelectSearchTeam: (teamId: string) => void;
  }) => {
    const { t } = useTranslation();
    return (
      <div>
        <button type="button" onClick={onSave}>
          {t("common.save")}
        </button>
        <span>Header {activeTabLabel}</span>
        <button type="button" onClick={onBack}>
          header-back
        </button>
        <button type="button" onClick={() => onSelectSearchPlayer("player-1")}>
          search-player
        </button>
        <button type="button" onClick={() => onSelectSearchTeam("team-2")}>
          search-team
        </button>
      </div>
    );
  },
}));

vi.mock("../components/playerProfile/PlayerProfile", () => ({
  default: ({
    onClose,
    onSelectTeam,
  }: {
    onClose: () => void;
    onSelectTeam: (teamId: string) => void;
  }) => (
    <div>
      <span>Player Profile Mock</span>
      <button type="button" onClick={onClose}>
        player-close
      </button>
      <button type="button" onClick={() => onSelectTeam("team-2")}>
        player-select-team
      </button>
    </div>
  ),
}));

vi.mock("../components/teamProfile", () => ({
  default: ({
    onClose,
    onSelectPlayer,
  }: {
    onClose: () => void;
    onSelectPlayer: (playerId: string) => void;
  }) => (
    <div>
      <span>Team Profile Mock</span>
      <button type="button" onClick={onClose}>
        team-close
      </button>
      <button type="button" onClick={() => onSelectPlayer("player-1")}>
        team-select-player
      </button>
    </div>
  ),
}));

vi.mock("../components/dashboard/DashboardAlerts", () => ({
  default: ({ alerts }: { alerts: { id: string }[] }) => (
    <div>
      Alerts Mock
      {alerts.map((alert) => (
        <span key={alert.id}>{alert.id}</span>
      ))}
    </div>
  ),
}));

vi.mock("../components/dashboard/DashboardTabContent", () => ({
  default: ({ viewModel }: { viewModel: { activeTab: string; seasonComplete: boolean } }) => (
    <div>
      <div>Tab Content {viewModel.activeTab}</div>
      {viewModel.seasonComplete ? <div>season over</div> : null}
    </div>
  ),
}));

vi.mock("../components/dashboard/DashboardBlockerModal", () => ({
  default: () => null,
}));

describe("Dashboard", () => {
  beforeEach(async () => {
    await i18nReady;
    await i18n.changeLanguage("en");
    isDirty = false;
    closeRequested = undefined;
    destroyMock.mockReset();
    gameState = createGameState();
    backendFinanceSnapshot = createBackendFinanceSnapshot();
    matchConfirmState.visible = false;
    registeredEventHandlers.clear();
    listenMock.mockClear();
    invokeMock.mockReset();
    setGameStateMock.mockReset();
    clearGameMock.mockReset();
    markCleanMock.mockReset();
    loadSettingsMock.mockReset();
    navigateMock.mockReset();
    vi.mocked(applyExtraTranslations).mockReset();
    window.localStorage.clear();
    gameState.competitions = undefined;
    gameState.season_context = undefined;
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_active_game") {
        return gameState;
      }

      if (command === "get_finance_snapshot") {
        return { snapshot: backendFinanceSnapshot };
      }

      return null;
    });
  });

  function refuseCommand(commandToRefuse: string, error: unknown) {
    const original = invokeMock.getMockImplementation();
    invokeMock.mockImplementation((command: string, ...args: unknown[]) => {
      if (command === commandToRefuse) return Promise.reject(error);
      return original?.(command, ...args);
    });
  }

  async function requestWindowClose() {
    const preventDefault = vi.fn();
    await act(async () => closeRequested?.({ preventDefault }));
    expect(preventDefault).toHaveBeenCalledOnce();
  }

  for (const { code } of SUPPORTED_LANGUAGES) {
    /** Given a live-match save refusal, when saving in this locale, then the real i18n hook shows the reason and retains the game. */
    it(`explains a refused save in ${code}`, async () => {
      await i18n.changeLanguage(code);
      render(<Dashboard />);
      refuseCommand("save_game", "be.error.liveMatch.inProgress");
      fireEvent.click(screen.getByRole("button", { name: i18n.t("common.save") }));
      const alert = await screen.findByRole("alert");
      expect(alert).toHaveTextContent(i18n.t("be.error.liveMatch.inProgress"));
      expect(alert).not.toHaveTextContent("be.error.");
      expect(markCleanMock).not.toHaveBeenCalled();
      expect(clearGameMock).not.toHaveBeenCalled();
      expect(navigateMock).not.toHaveBeenCalled();
    });

    /** Given a dirty career with a live match, when Save & Quit is refused, then the window and game remain and another close is intercepted. */
    it(`keeps the window after a refused Save & Quit in ${code}`, async () => {
      await i18n.changeLanguage(code);
      isDirty = true;
      render(<Dashboard />);
      refuseCommand("save_game", "be.error.liveMatch.inProgress");
      await requestWindowClose();
      fireEvent.click(screen.getByRole("button", { name: i18n.t("closeConfirm.saveQuit") }));
      const alert = await screen.findByRole("alert");
      expect(alert).toHaveTextContent(i18n.t("be.error.liveMatch.inProgress"));
      expect(alert).not.toHaveTextContent("be.error.");
      expect(destroyMock).not.toHaveBeenCalled();
      expect(clearGameMock).not.toHaveBeenCalled();
      expect(markCleanMock).not.toHaveBeenCalled();
      await requestWindowClose();
      expect(
        screen.getByRole("button", { name: i18n.t("closeConfirm.saveQuit") }),
      ).toBeInTheDocument();
    });

    /** Given a live match, when Save & Exit is refused, then the translated reason is shown, the game remains and exit can be retried. */
    it(`keeps the career after a refused Save & Exit in ${code}`, async () => {
      await i18n.changeLanguage(code);
      render(<Dashboard />);
      refuseCommand("exit_to_menu", "be.error.liveMatch.inProgress");
      fireEvent.click(screen.getByRole("button", { name: "exit-menu" }));
      fireEvent.click(screen.getByRole("button", { name: i18n.t("exitConfirm.saveExit") }));
      const alert = await screen.findByRole("alert");
      expect(alert).toHaveTextContent(i18n.t("be.error.liveMatch.inProgress"));
      expect(alert).not.toHaveTextContent("be.error.");
      expect(clearGameMock).not.toHaveBeenCalled();
      expect(navigateMock).not.toHaveBeenCalled();
      expect(screen.queryByText(i18n.t("exitConfirm.savingTitle"))).not.toBeInTheDocument();
      invokeMock.mockResolvedValue(null);
      fireEvent.click(screen.getByRole("button", { name: "exit-menu" }));
      fireEvent.click(screen.getByRole("button", { name: i18n.t("exitConfirm.saveExit") }));
      await waitFor(() => expect(navigateMock).toHaveBeenCalledWith("/"));
      expect(clearGameMock).toHaveBeenCalledOnce();
    });
  }

  /** Given a dirty career, when Save & Quit succeeds, then saving precedes closing the window. */
  it("closes the window after a successful save", async () => {
    isDirty = true;
    render(<Dashboard />);
    let finishSave: (() => void) | undefined;
    const save = new Promise<void>((resolve) => {
      finishSave = resolve;
    });
    const original = invokeMock.getMockImplementation();
    invokeMock.mockImplementation((command, ...args: unknown[]) =>
      command === "save_game" ? save : original?.(command, ...args),
    );
    await requestWindowClose();
    fireEvent.click(screen.getByRole("button", { name: "Save & Quit" }));
    expect(invokeMock).toHaveBeenCalledWith("save_game");
    expect(destroyMock).not.toHaveBeenCalled();
    expect(markCleanMock).not.toHaveBeenCalled();
    await act(async () => finishSave?.());
    await waitFor(() => expect(destroyMock).toHaveBeenCalledOnce());
    expect(markCleanMock).toHaveBeenCalledOnce();
  });

  /** Given a dirty career, when the player explicitly chooses Quit Without Saving, then no save is attempted and the window closes. */
  it("honors an explicit quit without saving", async () => {
    isDirty = true;
    render(<Dashboard />);
    await requestWindowClose();
    fireEvent.click(screen.getByRole("button", { name: "Quit Without Saving" }));
    await waitFor(() => expect(destroyMock).toHaveBeenCalledOnce());
    expect(invokeMock).not.toHaveBeenCalledWith("save_game");
    expect(markCleanMock).not.toHaveBeenCalled();
  });

  /** Given a disk failure, when Save & Quit fails, then the error is visible and the unsaved career stays open. */
  it("keeps the window after another save failure", async () => {
    isDirty = true;
    render(<Dashboard />);
    refuseCommand("save_game", new Error("disk unavailable"));
    await requestWindowClose();
    fireEvent.click(screen.getByRole("button", { name: "Save & Quit" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("disk unavailable");
    expect(destroyMock).not.toHaveBeenCalled();
    expect(markCleanMock).not.toHaveBeenCalled();
  });

  it("uses the backend cash verdict instead of a false local crisis", async () => {
    const team = gameState.teams[0];
    const player = gameState.players[0];
    if (!team || !player) throw new Error("expected managed team and player");
    team.finance = 20000;
    team.wage_budget = 20000;
    player.wage = 10000;
    backendFinanceSnapshot = {
      ...backendFinanceSnapshot,
      weekly_recurring_income: 15200,
      projected_weekly_net: 5200,
      wage_budget_status: "warning",
      wage_budget_usage_percent: 105,
      overall_status: "warning",
    };

    render(<Dashboard />);

    expect(await screen.findByText("wage_pressure")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("get_finance_snapshot", { teamId: "team-1" });
    expect(screen.queryByText("finance_crisis")).not.toBeInTheDocument();
  });

  it("shows a finance crisis when the backend verdict is critical", async () => {
    backendFinanceSnapshot = {
      ...backendFinanceSnapshot,
      runway_status: "critical",
      cash_runway_weeks: 2,
      overall_status: "critical",
    };

    render(<Dashboard />);

    expect(await screen.findByText("finance_crisis")).toBeInTheDocument();
  });

  it("keeps the last verdict during a same-club refetch but drops it for a different club", async () => {
    backendFinanceSnapshot = {
      ...backendFinanceSnapshot,
      runway_status: "critical",
      cash_runway_weeks: 2,
      overall_status: "critical",
    };
    const pendingFinanceResponses: Array<
      (response: { snapshot: ReturnType<typeof createBackendFinanceSnapshot> }) => void
    > = [];
    let financeRequestCount = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_active_game") return Promise.resolve(gameState);
      if (command === "get_finance_snapshot") {
        financeRequestCount += 1;
        if (financeRequestCount === 1) {
          return Promise.resolve({ snapshot: backendFinanceSnapshot });
        }
        return new Promise<{ snapshot: ReturnType<typeof createBackendFinanceSnapshot> }>(
          (resolve) => pendingFinanceResponses.push(resolve),
        );
      }
      return Promise.resolve(null);
    });

    const { rerender } = render(<Dashboard />);
    expect(await screen.findByText("finance_crisis")).toBeInTheDocument();

    gameState = createGameState();
    rerender(<Dashboard />);
    await waitFor(() => expect(pendingFinanceResponses).toHaveLength(1));
    expect(screen.getByText("finance_crisis")).toBeInTheDocument();

    gameState = createGameState();
    gameState.manager.team_id = "team-2";
    rerender(<Dashboard />);
    await waitFor(() => expect(pendingFinanceResponses).toHaveLength(2));
    expect(screen.queryByText("finance_crisis")).not.toBeInTheDocument();
  });

  it("ignores stale finance responses after the game state changes", async () => {
    const firstGameState = gameState;
    const pendingFinanceResponses: Array<
      (response: { snapshot: ReturnType<typeof createBackendFinanceSnapshot> }) => void
    > = [];
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_active_game") return Promise.resolve(gameState);
      if (command === "get_finance_snapshot") {
        return new Promise<{ snapshot: ReturnType<typeof createBackendFinanceSnapshot> }>(
          (resolve) => pendingFinanceResponses.push(resolve),
        );
      }
      return Promise.resolve(null);
    });

    const criticalResponse = {
      snapshot: {
        ...createBackendFinanceSnapshot(),
        runway_status: "critical",
        cash_runway_weeks: 2,
        overall_status: "critical",
      },
    };
    const { rerender } = render(<Dashboard />);
    await waitFor(() => expect(pendingFinanceResponses).toHaveLength(1));

    gameState = createGameState();
    gameState.manager.team_id = "team-2";
    rerender(<Dashboard />);
    await waitFor(() => expect(pendingFinanceResponses).toHaveLength(2));
    await act(async () => pendingFinanceResponses[1]?.(criticalResponse));
    expect(screen.getByText("finance_crisis")).toBeInTheDocument();

    // A verdict for the other club must disappear while this club refetches.
    gameState = firstGameState;
    rerender(<Dashboard />);
    await waitFor(() => expect(pendingFinanceResponses).toHaveLength(3));
    expect(screen.queryByText("finance_crisis")).not.toBeInTheDocument();

    // Returning to the original state also checks that its cancelled request
    // cannot overwrite the current verdict while the new request is pending.
    await act(async () => pendingFinanceResponses[0]?.(criticalResponse));
    expect(screen.queryByText("finance_crisis")).not.toBeInTheDocument();
  });

  // A finished league that is not the player's own, sorting first in the array.
  // Competitions are ordered by country code, so in a generated world this is
  // always Argentina — a split-season country on a different calendar.
  function foreignLeagueStillPlaying(): LeagueData {
    return {
      id: "ar-d1-apertura",
      name: "ar-d1-apertura",
      season: 2035,
      kind: "League",
      scope: "Domestic",
      country_id: "AR",
      priority: 0,
      participant_ids: ["ar-00", "ar-01"],
      rules: { format: "LeagueTable", counts_in_season_flow: true },
      fixtures: [
        {
          id: "ar-1",
          matchday: 1,
          date: "2036-02-10",
          home_team_id: "ar-00",
          away_team_id: "ar-01",
          competition: "League",
          status: "Completed",
          result: null,
        },
        {
          id: "ar-2",
          matchday: 2,
          date: "2036-10-10",
          home_team_id: "ar-01",
          away_team_id: "ar-00",
          competition: "League",
          status: "Scheduled",
          result: null,
        },
      ],
      standings: [],
    } as LeagueData;
  }

  function seasonContext(seasonComplete: boolean): SeasonContextData {
    return {
      phase: seasonComplete ? "PostSeason" : "InSeason",
      season_complete: seasonComplete,
      season_start: null,
      season_end: null,
      days_until_season_start: null,
      transfer_window: {
        status: "Closed",
        opens_on: null,
        closes_on: null,
        days_until_opens: null,
        days_remaining: null,
      },
    } as SeasonContextData;
  }

  /// The end-of-season screen is the only way to roll the season over, and the
  /// rollover is what runs promotion and relegation. This used to be decided
  /// from `competitions[0]` — never the player's league — so an English career
  /// that finished in April waited on an Argentine Apertura running to October,
  /// the screen never appeared, and the player could not continue.
  it("offers the end of season from the backend flag, not the first competition", async () => {
    gameState.competitions = [foreignLeagueStillPlaying()];
    gameState.season_context = seasonContext(true);

    render(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText("season over")).toBeInTheDocument();
    });
  });

  it("keeps the season running while the backend says it is not over", async () => {
    // The first competition has finished every fixture, which is exactly what
    // the old local rule keyed on. The backend says otherwise.
    const finished = foreignLeagueStillPlaying();
    finished.fixtures = finished.fixtures.map((fixture) => ({
      ...fixture,
      status: "Completed",
    }));
    gameState.competitions = [finished];
    gameState.season_context = seasonContext(false);

    render(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText("Tab Content Home")).toBeInTheDocument();
    });
    expect(screen.queryByText("season over")).not.toBeInTheDocument();
  });

  it("shows the localized named cup in the match confirmation modal", async () => {
    matchConfirmState.visible = true;
    i18n.addResource("en", "translation", "tournaments.competitions.nationalCup", "Copa Nacional");
    gameState.competitions = [
      {
        id: "cup-1",
        name: "National Cup",
        name_key: "tournaments.competitions.nationalCup",
        season: 2026,
        fixtures: [
          {
            id: "cup-match",
            competition_id: "cup-1",
            competition: "Cup",
            matchday: 1,
            date: "2026-07-10",
            home_team_id: "team-1",
            away_team_id: "team-2",
            status: "Scheduled",
            result: null,
          },
        ],
        standings: [],
      },
    ];

    render(<Dashboard />);

    expect(await screen.findByText("Copa Nacional")).toBeInTheDocument();
    expect(screen.queryByText("National Cup")).not.toBeInTheDocument();
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

  it("loads game state when the active save id fetch fails", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_active_game") {
        return gameState;
      }

      if (command === "get_finance_snapshot") {
        return { snapshot: backendFinanceSnapshot };
      }

      if (command === "get_active_save_id") {
        throw new Error("save id unavailable");
      }

      return null;
    });

    render(<Dashboard />);

    await waitFor(() => {
      expect(setGameStateMock).toHaveBeenCalledWith(gameState);
    });
    expect(applyExtraTranslations).toHaveBeenCalledWith(gameState.extra_translations);
    expect(navigateMock).not.toHaveBeenCalled();
    expect(clearGameMock).not.toHaveBeenCalled();
  });

  it("clears a stale active save id when a later refresh cannot load it", async () => {
    const saveStorageKey = "ofm-onboarding-visited-tabs:save:save-1";
    const legacyStorageKey = `ofm-onboarding-visited-tabs:legacy:${gameState.manager.id}:${gameState.clock.start_date}`;
    const getItemSpy = vi.spyOn(Storage.prototype, "getItem");
    let saveIdRequestCount = 0;

    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_active_game") {
        return gameState;
      }

      if (command === "get_finance_snapshot") {
        return { snapshot: backendFinanceSnapshot };
      }

      if (command === "get_active_save_id") {
        saveIdRequestCount += 1;
        if (saveIdRequestCount === 1) {
          return "save-1";
        }

        throw new Error("save id unavailable");
      }

      return null;
    });

    render(<Dashboard />);

    await waitFor(() => {
      expect(getItemSpy).toHaveBeenCalledWith(saveStorageKey);
    });

    getItemSpy.mockClear();
    const gameStateChangedHandler = registeredEventHandlers.get("game-state-changed");
    expect(gameStateChangedHandler).toBeTypeOf("function");

    await gameStateChangedHandler?.();

    await waitFor(() => {
      expect(getItemSpy).toHaveBeenCalledWith(legacyStorageKey);
    });

    getItemSpy.mockRestore();
  });
});
