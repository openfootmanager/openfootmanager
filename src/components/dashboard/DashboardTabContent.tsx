import { lazy, Suspense } from "react";
import type { JSX } from "react";
import type { DashboardTabContentModel } from "./dashboardTabContentModel";

const HomeTab = lazy(() => import("../home/HomeTab"));
const SquadTab = lazy(() => import("../squad/SquadTab"));
const TacticsTab = lazy(() => import("../tactics/TacticsTab"));
const TrainingTab = lazy(() => import("../training/TrainingTab"));
const ScheduleTab = lazy(() => import("../schedule/ScheduleTab"));
const FinancesTab = lazy(() => import("../finances/FinancesTab"));
const TransfersTab = lazy(() => import("../transfers/TransfersTab"));
const PlayersListTab = lazy(() => import("../players/PlayersListTab"));
const TeamsListTab = lazy(() => import("../teams/TeamsListTab"));
const TournamentsTab = lazy(() => import("../tournaments/TournamentsTab"));
const ScoutingTab = lazy(() => import("../scouting/ScoutingTab"));
const YouthAcademyTab = lazy(() => import("../youthAcademy/YouthAcademyTab"));
const StaffTab = lazy(() => import("../staff/StaffTab"));
const InboxTab = lazy(() => import("../inbox/InboxTab"));
const ManagerTab = lazy(() => import("../manager/ManagerTab"));
const NewsTab = lazy(() => import("../news/NewsTab"));
const EndOfSeasonScreen = lazy(() => import("../EndOfSeasonScreen"));

function DashboardTabFallback(): JSX.Element {
  return (
    <div className="flex min-h-[18rem] items-center justify-center rounded-2xl border border-gray-200/70 bg-white/70 dark:border-navy-700 dark:bg-navy-800/60">
      <div className="h-8 w-8 rounded-full border-4 border-primary-500 border-t-transparent animate-spin" />
    </div>
  );
}

interface DashboardTabContentProps {
  viewModel: DashboardTabContentModel;
}

export default function DashboardTabContent({
  viewModel,
}: DashboardTabContentProps): JSX.Element | null {
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

  let content: JSX.Element | null = null;

  switch (activeTab) {
    case "Home":
      content = seasonComplete ? (
        <EndOfSeasonScreen gameState={gameState} onGameUpdate={onGameUpdate} />
      ) : (
        <HomeTab
          gameState={gameState}
          onNavigate={onNavigate}
          visitedOnboardingTabs={visitedOnboardingTabs}
        />
      );
      break;
    case "Squad":
      content = (
        <SquadTab
          gameState={gameState}
          managerId={managerId}
          onSelectPlayer={onSelectPlayer}
          onGameUpdate={onGameUpdate}
        />
      );
      break;
    case "Tactics":
      content = (
        <TacticsTab
          gameState={gameState}
          onSelectPlayer={onSelectPlayer}
          onGameUpdate={onGameUpdate}
        />
      );
      break;
    case "Training":
      content = <TrainingTab gameState={gameState} onGameUpdate={onGameUpdate} />;
      break;
    case "Schedule":
      content = <ScheduleTab gameState={gameState} onSelectTeam={onSelectTeam} />;
      break;
    case "Finances":
      content = (
        <FinancesTab
          gameState={gameState}
          onGameUpdate={onGameUpdate}
          onSelectPlayer={onSelectPlayer}
        />
      );
      break;
    case "Transfers":
      content = (
        <TransfersTab
          gameState={gameState}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
          onGameUpdate={onGameUpdate}
        />
      );
      break;
    case "Players":
      content = (
        <PlayersListTab
          gameState={gameState}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
        />
      );
      break;
    case "Teams":
      content = <TeamsListTab gameState={gameState} onSelectTeam={onSelectTeam} />;
      break;
    case "Tournaments":
      content = <TournamentsTab gameState={gameState} onSelectTeam={onSelectTeam} />;
      break;
    case "Staff":
      content = <StaffTab gameState={gameState} onGameUpdate={onGameUpdate} />;
      break;
    case "Scouting":
      content = (
        <ScoutingTab
          gameState={gameState}
          onGameUpdate={onGameUpdate}
          onSelectPlayer={onSelectPlayer}
        />
      );
      break;
    case "Youth":
      content = (
        <YouthAcademyTab
          gameState={gameState}
          onSelectPlayer={onSelectPlayer}
        />
      );
      break;
    case "Inbox":
      content = (
        <InboxTab
          gameState={gameState}
          onGameUpdate={onGameUpdate}
          initialMessageId={initialMessageId}
          onNavigate={onNavigate}
        />
      );
      break;
    case "Manager":
      content = <ManagerTab gameState={gameState} />;
      break;
    case "News":
      content = <NewsTab gameState={gameState} onSelectTeam={onSelectTeam} />;
      break;
    default:
      content = null;
      break;
  }

  if (!content) {
    return null;
  }

  return <Suspense fallback={<DashboardTabFallback />}>{content}</Suspense>;
}
