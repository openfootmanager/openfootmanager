import type { TFunction } from "i18next";
import { formatDateShort } from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import type { GameStateData } from "../../store/gameStore";
import {
  resolveBoardObjective,
  resolveMessage,
  resolveNewsArticle,
} from "../../utils/backendI18n";
import {
  getHomeRosterOverview,
  getLeagueDigestArticles,
  getNextOpponentWidgetData,
  getOnboardingCompletionState,
  getRecentResultsForTeam,
} from "./HomeTab.helpers";

interface CreateHomeTabViewModelArgs {
  gameState: GameStateData;
  lang: string;
  t: TFunction;
  visitedOnboardingTabs: ReadonlySet<string>;
}

function compareStandings(
  leftEntry: { goals_against: number; goals_for: number; points: number },
  rightEntry: { goals_against: number; goals_for: number; points: number },
): number {
  return (
    rightEntry.points - leftEntry.points ||
    rightEntry.goals_for -
      rightEntry.goals_against -
      (leftEntry.goals_for - leftEntry.goals_against)
  );
}

export function createHomeTabViewModel({
  gameState,
  lang,
  t,
  visitedOnboardingTabs,
}: CreateHomeTabViewModelArgs) {
  const myTeam = gameState.teams.find(
    (team) => team.id === gameState.manager.team_id,
  );
  const league = gameState.league;
  const roster = myTeam
    ? gameState.players.filter((player) => player.team_id === myTeam.id)
    : [];
  const rosterOverview = getHomeRosterOverview(roster);
  const resolveInjuryName = (injuryName: string): string => {
    if (injuryName.includes(".")) {
      return t(injuryName, { defaultValue: injuryName });
    }

    return t(`common.injuries.${injuryName}`, { defaultValue: injuryName });
  };
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";
  const seasonStartLabel = seasonContext.season_start
    ? formatDateShort(seasonContext.season_start, lang)
    : null;
  const transferWindow = seasonContext.transfer_window;
  const transferWindowVariant: "danger" | "success" | "neutral" =
    transferWindow.status === "DeadlineDay"
      ? "danger"
      : transferWindow.status === "Open"
        ? "success"
        : "neutral";
  const transferWindowSummary =
    transferWindow.status === "DeadlineDay"
      ? t("season.windowClosesToday")
      : transferWindow.status === "Open" &&
          transferWindow.days_remaining !== null
        ? t("season.windowClosesInDays", {
            count: transferWindow.days_remaining,
          })
        : transferWindow.status === "Closed" &&
            transferWindow.days_until_opens !== null
          ? t("season.windowOpensInDays", {
              count: transferWindow.days_until_opens,
            })
          : t("season.windowClosed");
  const sortedStandings = league
    ? [...league.standings].sort(compareStandings)
    : [];
  const myStanding =
    !isPreseason && myTeam
      ? sortedStandings.findIndex((entry) => entry.team_id === myTeam.id) + 1 ||
        null
      : null;
  const myStandingData =
    !isPreseason && league && myTeam
      ? league.standings.find((entry) => entry.team_id === myTeam.id) ?? null
      : null;
  const recentResults = getRecentResultsForTeam(gameState, myTeam?.id ?? null);
  const schedule = myTeam?.training_schedule || "Balanced";
  const scheduleLabel = t(`common.trainingSchedules.${schedule}`, schedule);
  const focus = myTeam?.training_focus || "Physical";
  const latestNews = [...(gameState.news || [])]
    .sort((leftArticle, rightArticle) => {
      return rightArticle.date.localeCompare(leftArticle.date);
    })
    .slice(0, 2);
  const recentMessages = (gameState.messages || [])
    .slice(0, 4)
    .map(resolveMessage);
  const nextOpponent = getNextOpponentWidgetData(gameState);
  const leagueDigestArticles = getLeagueDigestArticles(gameState).map(
    resolveNewsArticle,
  );
  const boardObjectives = (gameState.board_objectives || []).map(
    resolveBoardObjective,
  );
  const onboardingState = getOnboardingCompletionState(
    gameState,
    visitedOnboardingTabs,
  );

  return {
    boardObjectives,
    boardObjectivesMetCount: boardObjectives.filter((objective) => objective.met)
      .length,
    focus,
    isPreseason,
    lang,
    latestNews,
    leagueDigestArticles,
    myStanding,
    myStandingData,
    nextOpponent,
    onboardingState,
    recentMessages,
    recentResults,
    resolveInjuryName,
    roster,
    rosterOverview,
    schedule,
    scheduleLabel,
    seasonContext,
    seasonStartLabel,
    teamForm: myTeam?.form ?? [],
    transferWindow,
    transferWindowSummary,
    transferWindowVariant,
  };
}

export type HomeTabViewModel = ReturnType<typeof createHomeTabViewModel>;