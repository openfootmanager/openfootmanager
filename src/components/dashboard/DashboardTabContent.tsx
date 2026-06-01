import { lazy, Suspense } from "react";
import EndOfSeasonScreen from "../EndOfSeasonScreen";
import HomeTab from "../home/HomeTab";
import type { DashboardTabContentModel } from "./dashboardTabContentModel";

const SquadTab = lazy(() => import("../squad/SquadTab"));
const TacticsTab = lazy(() => import("../tactics/TacticsTab"));
const TrainingTab = lazy(() => import("../training/TrainingTab"));
const ScheduleTab = lazy(() => import("../schedule/ScheduleTab"));
const FinancesTab = lazy(() => import("../finances/FinancesTab"));
const TransfersTab = lazy(() => import("../transfers/TransfersTab"));
const TransferCentreWorldTab = lazy(() => import("../transfers/TransferCentreWorldTab"));
const HallOfFameWorldTab = lazy(() => import("../hallOfFame/HallOfFameWorldTab"));
const PlayersListTab = lazy(() => import("../players/PlayersListTab"));
const ManagersWorldTab = lazy(() => import("../manager/ManagersWorldTab"));
const TeamsListTab = lazy(() => import("../teams/TeamsListTab"));
const TournamentsTab = lazy(() => import("../tournaments/TournamentsTab"));
const ScoutingTab = lazy(() => import("../scouting/ScoutingTab"));
const YouthAcademyTab = lazy(() => import("../youthAcademy/YouthAcademyTab"));
const StaffTab = lazy(() => import("../staff/StaffTab"));
const InboxTab = lazy(() => import("../inbox/InboxTab"));
const ManagerTab = lazy(() => import("../manager/ManagerTab"));
const NewsTab = lazy(() => import("../news/NewsTab"));

interface DashboardTabContentProps {
  viewModel: DashboardTabContentModel;
}

function DashboardTabFallback() {
  return (
    <div className="flex min-h-48 items-center justify-center">
      <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent" />
    </div>
  );
}

export default function DashboardTabContent({
  viewModel,
}: DashboardTabContentProps) {
  const {
    activeTab,
    gameState,
    initialMessageId,
    managerId,
    seasonComplete,
    visitedOnboardingTabs,
    handlers: {
      onGameUpdate,
      onNavigate,
      onSelectPlayer,
      onSelectTeam,
    },
  } = viewModel;

  const renderHomeContent = () => {
    if (seasonComplete) {
      return (
        <EndOfSeasonScreen gameState={gameState} onGameUpdate={onGameUpdate} />
      );
    }

    return (
      <HomeTab
        gameState={gameState}
        onNavigate={onNavigate}
        onGameUpdate={onGameUpdate}
        visitedOnboardingTabs={visitedOnboardingTabs}
      />
    );
  };

  let content = null;

  if (seasonComplete && activeTab === "Home") {
    content = renderHomeContent();
  } else if (activeTab === "Home") {
    content = renderHomeContent();
  } else if (activeTab === "Squad") {
    content = (
      <SquadTab
        gameState={gameState}
        managerId={managerId}
        onSelectPlayer={onSelectPlayer}
        onGameUpdate={onGameUpdate}
      />
    );
  } else if (activeTab === "Tactics") {
    content = (
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={onSelectPlayer}
        onGameUpdate={onGameUpdate}
      />
    );
  } else if (activeTab === "Training") {
    content = <TrainingTab gameState={gameState} onGameUpdate={onGameUpdate} />;
  } else if (activeTab === "Schedule") {
    content = <ScheduleTab gameState={gameState} onSelectTeam={onSelectTeam} />;
  } else if (activeTab === "Finances") {
    content = (
      <FinancesTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={onSelectPlayer}
      />
    );
  } else if (activeTab === "Transfers") {
    content = (
      <TransfersTab
        gameState={gameState}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
        onGameUpdate={onGameUpdate}
      />
    );
  } else if (activeTab === "TransferCentre") {
    content = (
      <TransferCentreWorldTab
        gameState={gameState}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />
    );
  } else if (activeTab === "HallOfFame") {
    content = (
      <HallOfFameWorldTab
        gameState={gameState}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />
    );
  } else if (activeTab === "Players") {
    content = (
      <PlayersListTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />
    );
  } else if (activeTab === "Managers") {
    content = (
      <ManagersWorldTab
        gameState={gameState}
        onSelectTeam={onSelectTeam}
      />
    );
  } else if (activeTab === "Teams") {
    content = <TeamsListTab gameState={gameState} onSelectTeam={onSelectTeam} />;
  } else if (activeTab === "Tournaments") {
    content = (
      <TournamentsTab
        gameState={gameState}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />
    );
  } else if (activeTab === "Staff") {
    content = <StaffTab gameState={gameState} onGameUpdate={onGameUpdate} />;
  } else if (activeTab === "Scouting") {
    content = (
      <ScoutingTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />
    );
  } else if (activeTab === "Youth") {
    content = (
      <YouthAcademyTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={onSelectPlayer}
      />
    );
  } else if (activeTab === "Inbox") {
    content = (
      <InboxTab
        gameState={gameState}
        onGameUpdate={onGameUpdate}
        initialMessageId={initialMessageId}
        onNavigate={onNavigate}
      />
    );
  } else if (activeTab === "Manager") {
    content = <ManagerTab gameState={gameState} onSelectTeam={onSelectTeam} />;
  } else if (activeTab === "News") {
    content = <NewsTab gameState={gameState} onSelectTeam={onSelectTeam} />;
  } else {
    console.warn("DashboardTabContent received unexpected activeTab", activeTab);
    content = renderHomeContent();
  }

  return (
    <Suspense fallback={<DashboardTabFallback />}>{content}</Suspense>
  );
}
