import type { JSX } from "react";
import {
  CheckCircle2,
  Circle,
  Crosshair,
  Dumbbell,
  Feather,
  Flame,
  Mail,
  Scale,
  UserCog,
  Users,
} from "lucide-react";
import type { TFunction } from "i18next";
import type { GameStateData } from "../../store/gameStore";
import NextMatchDisplay from "../NextMatchDisplay";
import { Badge, Card, CardBody, CardHeader } from "../ui";
import HomeLeagueDigestCard from "./HomeLeagueDigestCard";
import HomeLeaguePositionCard from "./HomeLeaguePositionCard";
import HomeLatestNewsCard from "./HomeLatestNewsCard";
import HomeNextOpponentCard from "./HomeNextOpponentCard";
import HomeOnboardingChecklistCard from "./HomeOnboardingChecklistCard";
import HomePlayerMomentumCard from "./HomePlayerMomentumCard";
import HomeRecentMessagesCard from "./HomeRecentMessagesCard";
import HomeRecentResultsCard from "./HomeRecentResultsCard";
import HomeSeasonStatusCard from "./HomeSeasonStatusCard";
import HomeSquadOverviewCard from "./HomeSquadOverviewCard";
import type { HomeTabViewModel } from "./HomeTab.viewModel";
import HomeUnavailablePlayersCard from "./HomeUnavailablePlayersCard";

const SCHEDULE_ICONS: Record<string, { color: string; icon: JSX.Element }> = {
  Intense: { icon: <Flame className="w-3.5 h-3.5" />, color: "text-red-500" },
  Balanced: {
    icon: <Scale className="w-3.5 h-3.5" />,
    color: "text-primary-500",
  },
  Light: {
    icon: <Feather className="w-3.5 h-3.5" />,
    color: "text-blue-500",
  },
};

interface HomeSectionProps {
  gameState: GameStateData;
  onNavigate?: (tab: string, context?: { messageId?: string }) => void;
  t: TFunction;
  viewModel: HomeTabViewModel;
}

function createOnboardingSteps(
  viewModel: HomeTabViewModel,
  t: TFunction,
): Array<{
  description: string;
  done: boolean;
  icon: JSX.Element;
  id: string;
  label: string;
  tab: string;
}> {
  return [
    {
      id: "squad",
      done: viewModel.onboardingState.hasVisitedSquadPage,
      label: t("onboarding.reviewSquad"),
      description: t("onboarding.reviewSquadDesc"),
      tab: "Squad",
      icon: <Users className="w-4 h-4" />,
    },
    {
      id: "staff",
      done: viewModel.onboardingState.hasVisitedStaffPage,
      label: t("onboarding.hireStaff"),
      description: t("onboarding.hireStaffDesc"),
      tab: "Staff",
      icon: <UserCog className="w-4 h-4" />,
    },
    {
      id: "tactics",
      done: viewModel.onboardingState.hasVisitedTacticsPage,
      label: t("onboarding.setTactics"),
      description: t("onboarding.setTacticsDesc"),
      tab: "Tactics",
      icon: <Crosshair className="w-4 h-4" />,
    },
    {
      id: "training",
      done: viewModel.onboardingState.hasVisitedTrainingPage,
      label: t("onboarding.configTraining"),
      description: t("onboarding.configTrainingDesc"),
      tab: "Training",
      icon: <Dumbbell className="w-4 h-4" />,
    },
    {
      id: "inbox",
      done: viewModel.onboardingState.hasReadInbox,
      label: t("onboarding.readMessages"),
      description: t("onboarding.readMessagesDesc"),
      tab: "Inbox",
      icon: <Mail className="w-4 h-4" />,
    },
  ];
}

export function HomeOverviewSection({
  gameState,
  onNavigate,
  t,
  viewModel,
}: HomeSectionProps): JSX.Element {
  const onboardingSteps = createOnboardingSteps(viewModel, t);
  const scheduleMeta =
    SCHEDULE_ICONS[viewModel.schedule] ?? SCHEDULE_ICONS.Balanced;

  return (
    <>
      {viewModel.isPreseason ? (
        <HomeSeasonStatusCard
          phase={viewModel.seasonContext.phase}
          seasonStartLabel={viewModel.seasonStartLabel}
          daysUntilSeasonStart={viewModel.seasonContext.days_until_season_start}
          transferWindowStatus={viewModel.transferWindow.status}
          transferWindowVariant={viewModel.transferWindowVariant}
          transferWindowSummary={viewModel.transferWindowSummary}
          transferWindowOpensOn={viewModel.transferWindow.opens_on}
          transferWindowClosesOn={viewModel.transferWindow.closes_on}
          lang={viewModel.lang}
        />
      ) : null}

      {viewModel.onboardingState.showOnboarding &&
      viewModel.onboardingState.completedSteps < onboardingSteps.length ? (
        <HomeOnboardingChecklistCard
          completedSteps={viewModel.onboardingState.completedSteps}
          totalSteps={onboardingSteps.length}
          steps={onboardingSteps}
          onNavigate={onNavigate}
        />
      ) : null}

      <div className="grid grid-cols-1 gap-5 md:grid-cols-3">
        <Card accent="primary" className="md:col-span-2">
          <CardHeader>{t("home.nextMatch")}</CardHeader>
          <CardBody>
            <NextMatchDisplay gameState={gameState} />
          </CardBody>
        </Card>

        <HomeLeaguePositionCard
          isPreseason={viewModel.isPreseason}
          phase={viewModel.seasonContext.phase}
          seasonStartLabel={viewModel.seasonStartLabel}
          myStanding={viewModel.myStanding}
          myStandingData={viewModel.myStandingData}
          teamForm={viewModel.teamForm}
          onNavigate={onNavigate}
        />
      </div>

      <div className="grid grid-cols-1 gap-5 md:grid-cols-2">
        <HomeNextOpponentCard
          nextOpponent={viewModel.nextOpponent}
          lang={viewModel.lang}
          onNavigate={onNavigate}
        />

        <HomeLeagueDigestCard
          articles={viewModel.leagueDigestArticles}
          lang={viewModel.lang}
          onNavigate={onNavigate}
        />
      </div>

      <HomeSquadOverviewCard
        avgCondition={viewModel.rosterOverview.avgCondition}
        avgOvr={viewModel.rosterOverview.avgOvr}
        exhaustedCount={viewModel.rosterOverview.exhaustedCount}
        scheduleIcon={scheduleMeta.icon}
        scheduleColorClass={scheduleMeta.color}
        scheduleLabel={viewModel.scheduleLabel}
        focus={viewModel.focus}
        onNavigate={onNavigate}
      />
    </>
  );
}

export function HomeBoardObjectivesSection({
  gameState,
  t,
  viewModel,
}: Pick<HomeSectionProps, "gameState" | "t" | "viewModel">): JSX.Element | null {
  if (viewModel.boardObjectives.length === 0) {
    return null;
  }

  return (
    <Card>
      <CardHeader>{t("manager.boardStatus", "Board Objectives")}</CardHeader>
      <CardBody>
        <div className="flex flex-col gap-2.5">
          {viewModel.boardObjectives.map((objective) => (
            <div key={objective.id} className="flex items-center gap-3">
              {objective.met ? (
                <CheckCircle2 className="w-4 h-4 text-green-500 flex-shrink-0" />
              ) : (
                <Circle className="w-4 h-4 text-gray-300 dark:text-navy-600 flex-shrink-0" />
              )}
              <span
                className={`text-sm ${objective.met ? "text-green-600 dark:text-green-400 line-through" : "text-gray-700 dark:text-gray-300"}`}
              >
                {objective.description}
              </span>
              <Badge
                variant={objective.met ? "success" : "neutral"}
                size="sm"
                className="ml-auto"
              >
                {objective.met ? t("home.met") : t("home.inProgress")}
              </Badge>
            </div>
          ))}
        </div>
        <div className="mt-3 border-t border-gray-100 pt-2 dark:border-navy-700">
          <p className="text-[10px] text-gray-400 dark:text-gray-500">
            {t("home.objectivesMet", {
              done: viewModel.boardObjectivesMetCount,
              total: viewModel.boardObjectives.length,
              pct: gameState.manager.satisfaction,
            })}
          </p>
        </div>
      </CardBody>
    </Card>
  );
}

export function HomeSquadPulseSection({
  gameState,
  onNavigate,
  viewModel,
}: Pick<HomeSectionProps, "gameState" | "onNavigate" | "viewModel">): JSX.Element {
  return (
    <>
      <HomeUnavailablePlayersCard
        players={viewModel.rosterOverview.unavailablePlayers}
        resolveInjuryName={viewModel.resolveInjuryName}
        onNavigate={onNavigate}
      />

      <div className="grid grid-cols-1 gap-5 md:grid-cols-2">
        <HomeRecentResultsCard
          recentResults={viewModel.recentResults}
          teams={gameState.teams}
          onNavigate={onNavigate}
        />

        <HomeLatestNewsCard
          articles={viewModel.latestNews}
          teams={gameState.teams}
          lang={viewModel.lang}
          onNavigate={onNavigate}
        />
      </div>

      {viewModel.roster.length > 0 &&
      (viewModel.rosterOverview.hotPlayers.length > 0 ||
        viewModel.rosterOverview.coldPlayers.length > 0) ? (
        <HomePlayerMomentumCard
          hotPlayers={viewModel.rosterOverview.hotPlayers}
          coldPlayers={viewModel.rosterOverview.coldPlayers}
          onNavigate={onNavigate}
        />
      ) : null}

      <HomeRecentMessagesCard
        messages={viewModel.recentMessages}
        lang={viewModel.lang}
        onNavigate={onNavigate}
      />
    </>
  );
}