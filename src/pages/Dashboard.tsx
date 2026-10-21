import { routeForActiveCareer } from "../services/careerService";
import { useCallback, useEffect, useRef, useState } from "react";
import { applyExtraTranslations } from "../lib/extraTranslations";
import type { JSX } from "react";
import { useNavigate } from "react-router-dom";
import { getActiveGame, getActiveSaveId, saveGame, exitToMenu } from "../services/sessionService";
import { resolveBackendError } from "../utils/backendI18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import type { MatchModeType } from "../hooks/useAdvanceTime";
import { useGameStore } from "../store/gameStore";
import type { GameStateData, PlayerSelectionOptions } from "../store/gameStore";
import DashboardHeader, {
  type DashboardMatchModeMeta,
} from "../components/dashboard/DashboardHeader";
import DashboardOverlays from "../components/dashboard/DashboardOverlays";
import FiredModal from "../components/dashboard/FiredModal";
import DashboardSidebar from "../components/dashboard/DashboardSidebar";
import DashboardWorkspaceContent from "../components/dashboard/DashboardWorkspaceContent";
import {
  createDashboardProfileNavigationState,
  goBackDashboardProfile,
  hasDashboardProfileHistory,
  navigateDashboardProfiles,
  openDashboardSearchPlayer,
  openDashboardSearchTeam,
  openProfileTitle,
  selectDashboardPlayer,
  selectDashboardTeam,
  type DashboardNavigateContext,
} from "../components/dashboard/dashboardProfileNavigation";
import { createDashboardTabContentModel } from "../components/dashboard/dashboardTabContentModel";
import {
  DEFAULT_SQUAD_LIST_SORT_STATE,
  type SquadListSortState,
} from "../components/squad/SquadRosterView.state";
import {
  isOnboardingPageTab,
  loadVisitedOnboardingTabs,
  saveVisitedOnboardingTabs,
} from "../components/home/HomeTab.helpers";
import {
  getDashboardAlerts,
  getDashboardSearchResults,
  clubCompetitionLabel,
  getManagerTeamName,
  getTodayMatchFixture,
  getUnreadMessagesCount,
} from "../components/dashboard/dashboardHelpers";
import { useAdvanceTime } from "../hooks/useAdvanceTime";
import { Cpu, Eye, Gamepad2 } from "lucide-react";
import { formatDate, formatDateFull, getFixtureCompetitionName } from "../lib/helpers";
import { useSidebarMustCollapse } from "../components/dashboard/sidebarAutoCollapse";
import { useTranslation } from "react-i18next";
import { useClubLeagueStanding } from "../components/teamProfile/useClubLeagueStanding";
import { useSettingsStore } from "../store/settingsStore";
import {
  getBackgroundPortraitPrewarmKey,
  queueBackgroundPortraitPrewarm,
} from "../services/portraitService";
import { getFinanceSnapshot, type TeamFinanceSnapshotData } from "../services/financeService";
import { isSeasonComplete } from "../lib/seasonContext";

const CLUB_TABS = new Set([
  "Squad",
  "Tactics",
  "Training",
  "Staff",
  "Scouting",
  "Youth",
  "Finances",
  "Transfers",
]);

const TAB_TRANSLATION_KEYS: Record<string, string> = {
  Home: "dashboard.home",
  Inbox: "dashboard.inbox",
  Manager: "dashboard.manager",
  Squad: "dashboard.squad",
  Tactics: "dashboard.tactics",
  Training: "dashboard.training",
  Staff: "dashboard.staff",
  Finances: "dashboard.finances",
  Transfers: "dashboard.transfers",
  TransferCentre: "transfers.centre",
  Players: "dashboard.players",
  Managers: "dashboard.managers",
  HallOfFame: "dashboard.hallOfFame",
  Teams: "dashboard.teams",
  Tournaments: "dashboard.tournaments",
  Schedule: "dashboard.schedule",
  News: "dashboard.news",
  Scouting: "dashboard.scouting",
  Youth: "dashboard.youthAcademy",
};

interface DashboardProps {
  /** The tab it opens on. The app always opens on Home; the dev page harness mounts other pages. */
  initialTab?: string;
}

export default function Dashboard({ initialTab = "Home" }: DashboardProps): JSX.Element {
  const navigate = useNavigate();
  const {
    hasActiveGame,
    managerName,
    gameState,
    sessionState,
    setGameState,
    clearGame,
    isDirty,
    markClean,
  } = useGameStore();
  const { t } = useTranslation();
  const { settings, loaded: settingsLoaded, loadSettings } = useSettingsStore();
  const latestGameStateRef = useRef<GameStateData | null>(null);
  latestGameStateRef.current = gameState ?? null;

  // Load settings on mount
  useEffect(() => {
    if (!settingsLoaded) loadSettings();
  }, [settingsLoaded, loadSettings]);
  const [isSaving, setIsSaving] = useState(false);
  const [saveFlash, setSaveFlash] = useState(false);
  const [saveError, setSaveError] = useState<unknown>(null);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  // Layout rule L5: below a 40rem workspace the sidebar folds to its icon rail, whatever the player chose.
  const sidebarMustCollapse = useSidebarMustCollapse();
  const [profileNavigation, setProfileNavigation] = useState(() =>
    createDashboardProfileNavigationState(initialTab),
  );
  const [showExitConfirm, setShowExitConfirm] = useState(false);
  const [isExitingToMenu, setIsExitingToMenu] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [visitedOnboardingTabs, setVisitedOnboardingTabs] = useState<Set<string>>(
    new Set<string>(),
  );
  const [activeSaveId, setActiveSaveId] = useState<string | null>(null);
  const [squadListSortState, setSquadListSortState] = useState<SquadListSortState>(
    DEFAULT_SQUAD_LIST_SORT_STATE,
  );
  const [financeVerdict, setFinanceVerdict] = useState<{
    teamId: string;
    snapshot: TeamFinanceSnapshotData;
  } | null>(null);

  useEffect(() => {
    const teamId = gameState?.manager.team_id;
    if (!gameState || !teamId) {
      setFinanceVerdict(null);
      return;
    }

    let cancelled = false;
    void getFinanceSnapshot(teamId)
      .then(({ snapshot }) => {
        if (!cancelled) {
          setFinanceVerdict({ teamId, snapshot });
        }
      })
      .catch((error) => {
        console.error("Failed to load dashboard finance snapshot:", error);
        if (!cancelled) {
          setFinanceVerdict(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [gameState]);
  const loadActiveGameState = useCallback(async () => {
    const [stateResult, saveIdResult] = await Promise.allSettled([
      getActiveGame(),
      getActiveSaveId(),
    ]);

    if (stateResult.status === "rejected") {
      throw stateResult.reason;
    }

    const activeState = stateResult.value;
    applyExtraTranslations(activeState.extra_translations);
    setGameState(activeState);

    if (saveIdResult.status === "fulfilled") {
      setActiveSaveId(saveIdResult.value);
    } else {
      setActiveSaveId(null);
    }

    const route = await routeForActiveCareer();
    if (route !== "/dashboard") {
      navigate(route);
    }
  }, [setGameState, navigate]);

  // Fetch initial state
  useEffect(() => {
    if (!hasActiveGame) {
      navigate("/");
      return;
    }

    const fetchState = async () => {
      try {
        await loadActiveGameState();
      } catch (err) {
        console.error("Failed to fetch game state:", err);
      }
    };

    fetchState();
  }, [hasActiveGame, loadActiveGameState, navigate]);

  // Refresh state when MCP tools mutate game (backend emits "game-state-changed")
  useEffect(() => {
    const unlisten = listen("game-state-changed", async () => {
      try {
        await loadActiveGameState();
      } catch {
        // Game may have been exited — navigate back to menu
        clearGame();
        navigate("/");
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadActiveGameState, clearGame, navigate]);

  const isUnemployed = gameState?.manager.team_id === null;
  const todayMatchFixture = gameState ? getTodayMatchFixture(gameState) : null;
  const todayMatchCompetitionName =
    gameState && todayMatchFixture
      ? getFixtureCompetitionName(gameState, todayMatchFixture, t)
      : undefined;
  const hasMatchToday = todayMatchFixture !== null;
  const backgroundPortraitPrewarmKey = gameState
    ? getBackgroundPortraitPrewarmKey(gameState)
    : null;

  useEffect(() => {
    if (!gameState) {
      return;
    }

    console.info("[Dashboard] matchDayStatus", {
      currentDate: gameState.clock.current_date,
      fixtureDate: todayMatchFixture?.date ?? null,
      fixtureId: todayMatchFixture?.id ?? null,
      fixtureStatus: todayMatchFixture?.status ?? null,
      hasMatchToday,
      managerTeamId: gameState.manager.team_id,
      matchMode: settings.default_match_mode,
    });
  }, [gameState, hasMatchToday, settings.default_match_mode, todayMatchFixture]);

  useEffect(() => {
    if (!gameState) {
      setVisitedOnboardingTabs(new Set<string>());
      return;
    }

    setVisitedOnboardingTabs(loadVisitedOnboardingTabs(gameState, undefined, activeSaveId));
  }, [gameState, activeSaveId]);

  useEffect(() => {
    setSquadListSortState(DEFAULT_SQUAD_LIST_SORT_STATE);
  }, [activeSaveId]);

  useEffect(() => {
    if (!backgroundPortraitPrewarmKey) {
      return;
    }

    const currentGameState = latestGameStateRef.current;
    if (!currentGameState) {
      return;
    }

    return queueBackgroundPortraitPrewarm(currentGameState);
  }, [backgroundPortraitPrewarmKey]);

  useEffect(() => {
    if (!isOnboardingPageTab(profileNavigation.activeTab)) {
      return;
    }

    if (!gameState) {
      return;
    }

    setVisitedOnboardingTabs((currentTabs) => {
      if (currentTabs.has(profileNavigation.activeTab)) {
        return currentTabs;
      }

      const nextTabs = new Set(currentTabs);
      nextTabs.add(profileNavigation.activeTab);
      saveVisitedOnboardingTabs(gameState, nextTabs, undefined, activeSaveId);
      return nextTabs;
    });
  }, [activeSaveId, gameState, profileNavigation.activeTab]);

  // Reset to Home tab if current tab is a club tab and manager is unemployed
  useEffect(() => {
    if (isUnemployed && profileNavigation.activeTab && CLUB_TABS.has(profileNavigation.activeTab)) {
      setProfileNavigation((s) => navigateDashboardProfiles(s, "Home"));
    }
  }, [isUnemployed, profileNavigation.activeTab]);

  // The backend decides this, because it is the same answer
  // `advance_to_next_season` enforces before it will roll the season over.
  // Deriving it here from `competitions[0]` asked whichever competition sorted
  // first — a foreign league on another calendar — so an English career whose
  // season ended in April waited on an Argentine Apertura running to October,
  // the end-of-season screen never appeared, and the player could not continue.
  const seasonComplete = gameState ? isSeasonComplete(gameState) : false;

  // Advance-time hook
  const {
    isAdvancing,
    showContinueMenu,
    setShowContinueMenu,
    showMatchConfirm,
    setShowMatchConfirm,
    matchMode,
    setMatchMode,
    blockerModal,
    setBlockerModal,
    handleContinue,
    handleConfirmMatch,
    handleSkipToMatchDay,
    digestEntries,
    digestStopReason,
    isDigestVisible,
    isDigestRunning,
    isDigestAborting,
    abortDigest,
    dismissDigest,
    resumeDigest,
    resumeAfterBlocker,
  } = useAdvanceTime(
    setGameState,
    hasMatchToday,
    settings.default_match_mode,
    settingsLoaded,
    isUnemployed ?? false,
    settings.continue_to_next_event,
  );

  const handleSave = useCallback(async () => {
    setIsSaving(true);
    setSaveError(null);
    setSaveFlash(false);
    try {
      await saveGame();
      markClean();
      setSaveFlash(true);
      setTimeout(() => setSaveFlash(false), 2000);
    } catch (err) {
      console.error("Failed to save:", err);
      setSaveError(err);
    } finally {
      setIsSaving(false);
    }
  }, [markClean]);

  // Intercept window close to warn about unsaved changes
  const [showCloseConfirm, setShowCloseConfirm] = useState(false);
  const isClosingRef = useRef(false);
  useEffect(() => {
    const appWindow = getCurrentWindow();
    const unlisten = appWindow.onCloseRequested(async (event) => {
      if (isClosingRef.current) return;
      if (isDirty) {
        event.preventDefault();
        setShowCloseConfirm(true);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isDirty]);

  const handleCloseQuit = async (save: boolean) => {
    setShowCloseConfirm(false);
    setSaveError(null);
    if (save) {
      try {
        await saveGame();
        markClean();
      } catch (err) {
        console.error("Auto-save on close failed:", err);
        setSaveError(err);
        return;
      }
    }
    isClosingRef.current = true;
    await getCurrentWindow().destroy();
  };

  const MODE_META: Record<MatchModeType, DashboardMatchModeMeta> = {
    live: {
      label: t("continueMenu.goToField"),
      icon: <Gamepad2 className="w-4 h-4" />,
      desc: t("continueMenu.goToFieldDesc"),
    },
    spectator: {
      label: t("continueMenu.watchSpectator"),
      icon: <Eye className="w-4 h-4" />,
      desc: t("continueMenu.watchSpectatorDesc"),
    },
    delegate: {
      label: t("continueMenu.delegateAssistant"),
      icon: <Cpu className="w-4 h-4" />,
      desc: t("continueMenu.delegateAssistantDesc"),
    },
  };

  const currentModeMeta = MODE_META[matchMode];

  function handleNavClick(tab: string): void {
    setProfileNavigation((currentState) => navigateDashboardProfiles(currentState, tab));
  }

  function handleNavigate(tab: string, context?: DashboardNavigateContext): void {
    setProfileNavigation((currentState) => navigateDashboardProfiles(currentState, tab, context));
  }

  function handleBack(): void {
    setProfileNavigation((currentState) => goBackDashboardProfile(currentState));
  }

  const handleExitToMenu = async () => {
    if (isExitingToMenu) {
      return;
    }

    setIsExitingToMenu(true);
    setSaveError(null);
    try {
      await exitToMenu();
      clearGame();
      navigate("/");
    } catch (err) {
      console.error("Failed to exit:", err);
      setSaveError(err);
    } finally {
      setIsExitingToMenu(false);
    }
  };

  function selectPlayer(id: string, options?: PlayerSelectionOptions): void {
    setProfileNavigation((currentState) => selectDashboardPlayer(currentState, id, options));
  }

  function selectTeam(id: string): void {
    setProfileNavigation((currentState) => selectDashboardTeam(currentState, id));
  }

  function handleSearchFocus(): void {
    setSearchOpen(true);
  }

  function handleSearchBlur(): void {
    setTimeout(() => setSearchOpen(false), 200);
  }

  function handleSearchQueryChange(query: string): void {
    setSearchQuery(query);
  }

  function handleSelectSearchPlayer(playerId: string): void {
    setProfileNavigation((currentState) => openDashboardSearchPlayer(currentState, playerId));
    setSearchQuery("");
  }

  function handleSelectSearchTeam(teamId: string): void {
    setProfileNavigation((currentState) => openDashboardSearchTeam(currentState, teamId));
    setSearchQuery("");
  }

  function handleToggleContinueMenu(): void {
    setShowContinueMenu(!showContinueMenu);
  }

  function handleSelectMatchMode(mode: MatchModeType): void {
    setMatchMode(mode);
    setShowContinueMenu(false);
  }

  function handleNavigateSettings(): void {
    navigate("/settings", { state: { from: "/dashboard" } });
  }

  const clubStanding = useClubLeagueStanding(
    gameState?.manager.team_id ?? "",
    gameState?.clock.current_date ?? "",
  );

  if (!gameState) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-page">
        <div className="flex flex-col items-center gap-3">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary-700 border-t-transparent motion-reduce:animate-none" />
          <span className="font-heading text-body uppercase tracking-label text-fg-muted">
            {t("dashboard.loading")}
          </span>
        </div>
      </div>
    );
  }

  const currentDate = formatDateFull(gameState.clock.current_date, settings.language);
  const currentDateShort = formatDate(gameState.clock.current_date, settings.language);
  const unreadMessagesCount =
    sessionState?.unread_messages_count ?? getUnreadMessagesCount(gameState);
  const myTeamName = getManagerTeamName(gameState);
  const myCompetitionName = clubCompetitionLabel(gameState, clubStanding.standing, t);
  const searchResults = getDashboardSearchResults(gameState, searchQuery);
  const currentFinanceVerdict =
    financeVerdict?.teamId === gameState.manager.team_id ? financeVerdict.snapshot : null;
  const dashboardAlerts = getDashboardAlerts(gameState, hasMatchToday, t, currentFinanceVerdict);
  const hasProfileHistory = hasDashboardProfileHistory(profileNavigation);
  const activeTabLabel = TAB_TRANSLATION_KEYS[profileNavigation.activeTab]
    ? t(TAB_TRANSLATION_KEYS[profileNavigation.activeTab])
    : profileNavigation.activeTab;
  const headerTitle =
    openProfileTitle(profileNavigation, gameState) ??
    (profileNavigation.selectedPlayerId !== null ? t("common.player") : activeTabLabel);
  const dashboardTabContentModel = createDashboardTabContentModel({
    activeTab: profileNavigation.activeTab,
    gameState,
    seasonComplete,
    visitedOnboardingTabs,
    squadListSortState,
    initialMessageId: profileNavigation.initialMessageId,
    handlers: {
      onSelectPlayer: selectPlayer,
      onSelectTeam: selectTeam,
      onGameUpdate: setGameState,
      onNavigate: handleNavigate,
      onSquadListSortChange: setSquadListSortState,
    },
  });

  return (
    <div className="flex min-h-screen bg-page">
      <DashboardSidebar
        activeTab={profileNavigation.activeTab}
        collapsed={isSidebarCollapsed || sidebarMustCollapse}
        collapseForced={sidebarMustCollapse}
        onNavClick={handleNavClick}
        onToggleCollapse={() => {
          setIsSidebarCollapsed((currentValue) => !currentValue);
        }}
        unreadMessagesCount={unreadMessagesCount}
        todayHasMatch={hasMatchToday}
        managerName={managerName}
        teamName={myTeamName}
        competitionName={myCompetitionName}
        onNavigateSettings={handleNavigateSettings}
        isUnemployed={isUnemployed ?? false}
        onExitClick={() => {
          if (!isExitingToMenu) {
            setShowExitConfirm(true);
          }
        }}
      />

      <DashboardOverlays
        blockerModal={blockerModal}
        currentModeMeta={currentModeMeta}
        isAdvancing={isAdvancing}
        handleConfirmMatch={handleConfirmMatch}
        handleExitToMenu={handleExitToMenu}
        handleNavigate={handleNavigate}
        handleCloseQuit={handleCloseQuit}
        isExitingToMenu={isExitingToMenu}
        matchMode={matchMode}
        setBlockerModal={setBlockerModal}
        setShowCloseConfirm={setShowCloseConfirm}
        setShowExitConfirm={setShowExitConfirm}
        setShowMatchConfirm={setShowMatchConfirm}
        showCloseConfirm={showCloseConfirm}
        showExitConfirm={showExitConfirm}
        showMatchConfirm={showMatchConfirm}
        teams={gameState.teams}
        todayMatchFixture={todayMatchFixture}
        todayMatchCompetitionName={todayMatchCompetitionName}
        digestEntries={digestEntries}
        digestStopReason={digestStopReason}
        isDigestVisible={isDigestVisible}
        isDigestRunning={isDigestRunning}
        isDigestAborting={isDigestAborting}
        onDigestContinueAfterBlocker={resumeAfterBlocker}
        onDigestResume={resumeDigest}
        onDismissDigest={dismissDigest}
        onDigestStop={abortDigest}
      />
      <FiredModal />

      {/* Main Content Area */}
      {/* min-w-0: a wide page (a table) must not widen <main>, and with it the window (layout rule L1). */}
      <main className="flex h-screen min-w-0 flex-1 flex-col overflow-hidden">
        {saveError !== null ? (
          <p
            role="alert"
            className="m-3 rounded-lg border border-line bg-warning-tint px-4 py-3 text-body text-warning"
          >
            {resolveBackendError(saveError)}
          </p>
        ) : null}
        <DashboardHeader
          title={headerTitle}
          currentDate={currentDate}
          currentDateShort={currentDateShort}
          hasProfileHistory={hasProfileHistory}
          hasMatchToday={hasMatchToday}
          isAdvancing={isAdvancing}
          isSaving={isSaving}
          matchMode={matchMode}
          matchedPlayers={searchResults.matchedPlayers}
          matchedTeams={searchResults.matchedTeams}
          modeMeta={MODE_META}
          onBack={handleBack}
          onContinue={handleContinue}
          onSave={handleSave}
          onSearchBlur={handleSearchBlur}
          onSearchFocus={handleSearchFocus}
          onSearchQueryChange={handleSearchQueryChange}
          onSelectMatchMode={handleSelectMatchMode}
          onSelectSearchPlayer={handleSelectSearchPlayer}
          onSelectSearchTeam={handleSelectSearchTeam}
          onSkipToMatchDay={handleSkipToMatchDay}
          onToggleContinueMenu={handleToggleContinueMenu}
          saveFlash={saveFlash}
          searchOpen={searchOpen}
          searchQuery={searchQuery}
          seasonComplete={seasonComplete}
          showContinueMenu={showContinueMenu}
          isUnemployed={isUnemployed ?? false}
          teams={gameState.teams}
        />

        <DashboardWorkspaceContent
          dashboardAlerts={dashboardAlerts}
          gameState={gameState}
          profileNavigation={profileNavigation}
          dashboardTabContentModel={dashboardTabContentModel}
          onNavigate={handleNavigate}
          onSelectPlayer={selectPlayer}
          onSelectTeam={selectTeam}
          onGameUpdate={setGameState}
          isUnemployed={isUnemployed ?? false}
        />
      </main>
    </div>
  );
}
