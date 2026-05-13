import type {
  FixtureData,
  GameStateData,
  PlayerData,
  TeamData,
} from "../../store/gameStore";
import { formatVal } from "../../lib/helpers";
import { getTeamFinanceSnapshot } from "../../lib/finance";
import { buildStartingXIIds } from "../squad/SquadTab.helpers";

export interface DashboardAlert {
  id: string;
  text: string;
  tab: string;
  severity: "warn" | "info";
}

export interface DashboardSearchResults {
  matchedPlayers: PlayerData[];
  matchedTeams: TeamData[];
}

type DashboardAlertTranslator = (
  key: string,
  options?: Record<string, unknown>,
) => string;

export function getTodayMatchFixture(gameState: GameStateData): FixtureData | null {
  const fixtures = gameState.league?.fixtures;

  if (!fixtures) {
    return null;
  }

  const today = gameState.clock.current_date.split("T")[0];

  return (
    fixtures.find((fixture) => {
      return (
        fixture.date === today &&
        fixture.status === "Scheduled" &&
        (fixture.home_team_id === gameState.manager.team_id ||
          fixture.away_team_id === gameState.manager.team_id)
      );
    }) ?? null
  );
}

export function getUnreadMessagesCount(gameState: GameStateData): number {
  return gameState.messages.filter((message) => !message.read).length;
}

export function getManagerTeamName(gameState: GameStateData): string | null {
  return (
    gameState.teams.find((team) => team.id === gameState.manager.team_id)?.name ??
    null
  );
}

export function getPlayerBadgeVariant(
  position: string,
): "accent" | "danger" | "primary" | "success" {
  switch (position) {
    case "Goalkeeper":
      return "accent";
    case "Defender":
      return "primary";
    case "Midfielder":
      return "success";
    default:
      return "danger";
  }
}

export function getDashboardSearchResults(
  gameState: GameStateData,
  query: string,
): DashboardSearchResults {
  const normalizedQuery = query.trim().toLowerCase();

  if (normalizedQuery.length < 2) {
    return {
      matchedPlayers: [],
      matchedTeams: [],
    };
  }

  return {
    matchedPlayers: gameState.players
      .filter((player) => {
        return (
          player.full_name.toLowerCase().includes(normalizedQuery) ||
          player.match_name.toLowerCase().includes(normalizedQuery)
        );
      })
      .slice(0, 5),
    matchedTeams: gameState.teams
      .filter((team) => {
        return (
          team.name.toLowerCase().includes(normalizedQuery) ||
          team.short_name.toLowerCase().includes(normalizedQuery)
        );
      })
      .slice(0, 4),
  };
}

export function getDashboardAlerts(
  gameState: GameStateData,
  hasMatchToday: boolean,
  t: DashboardAlertTranslator,
): DashboardAlert[] {
  const alerts: DashboardAlert[] = [];
  const myTeam = gameState.teams.find(
    (team) => team.id === gameState.manager.team_id,
  );
  const roster = myTeam
    ? gameState.players.filter((player) => player.team_id === myTeam.id)
    : [];
  const teamStaff = myTeam
    ? gameState.staff.filter((staffMember) => staffMember.team_id === myTeam.id)
    : [];
  const financeSnapshot = myTeam
    ? getTeamFinanceSnapshot(myTeam, roster, teamStaff)
    : null;
  const exhaustedCount = roster.filter((player) => player.condition < 25).length;
  const urgentUnreadCount = gameState.messages.filter((message) => {
    return !message.read && message.priority === "Urgent";
  }).length;
  const savedStartingXi = myTeam?.starting_xi_ids ?? [];
  const effectiveStartingXi = myTeam
    ? buildStartingXIIds(roster, savedStartingXi, myTeam.formation)
    : [];
  const xiPlayersOnRoster = effectiveStartingXi.filter((playerId) => {
    return roster.some((player) => player.id === playerId);
  });
  const injuredInXiCount = xiPlayersOnRoster.filter((playerId) => {
    return roster.find((player) => player.id === playerId)?.injury;
  }).length;
  const healthyXiCount = xiPlayersOnRoster.length - injuredInXiCount;

  if (exhaustedCount >= 3) {
    alerts.push({
      id: "exhausted",
      text: t("dashboard.alerts.exhausted", { count: exhaustedCount }),
      tab: "Training",
      severity: "warn",
    });
  }

  if (savedStartingXi.length > 0) {
    if (injuredInXiCount > 0) {
      alerts.push({
        id: "injured_xi",
        text: t("dashboard.alerts.injuredStartingXi", {
          count: injuredInXiCount,
        }),
        tab: "Squad",
        severity: "warn",
      });
    }

    if (
      healthyXiCount < 11 &&
      injuredInXiCount === 0 &&
      roster.length >= 11
    ) {
      alerts.push({
        id: "xi",
        text: t("dashboard.alerts.incompleteStartingXi"),
        tab: "Squad",
        severity: "warn",
      });
    }
  }

  if (urgentUnreadCount > 0) {
    alerts.push({
      id: "urgent",
      text: t("dashboard.alerts.urgentUnread", { count: urgentUnreadCount }),
      tab: "Inbox",
      severity: "warn",
    });
  }

  if (myTeam && financeSnapshot) {
    if (myTeam.finance < 0 || financeSnapshot.runwayStatus === "critical") {
      alerts.push({
        id: "finance_crisis",
        text: t("dashboard.alerts.financeCrisis", {
          balance: formatVal(myTeam.finance),
          weeks: financeSnapshot.cashRunwayWeeks ?? 0,
        }),
        tab: "Finances",
        severity: "warn",
      });
    } else if (financeSnapshot.runwayStatus === "warning") {
      alerts.push({
        id: "finance_runway",
        text: t("dashboard.alerts.financeRunway", {
          count: financeSnapshot.cashRunwayWeeks,
        }),
        tab: "Finances",
        severity: "warn",
      });
    }

    if (
      financeSnapshot.wageBudgetStatus === "warning" ||
      financeSnapshot.wageBudgetStatus === "critical"
    ) {
      alerts.push({
        id: "wage_pressure",
        text: t("dashboard.alerts.wagePressure", {
          percent: financeSnapshot.wageBudgetUsagePercent,
        }),
        tab: "Finances",
        severity: "warn",
      });
    }
  }

  if (hasMatchToday && savedStartingXi.length > 0 && healthyXiCount < 11) {
    alerts.push({
      id: "matchxi",
      text: t("dashboard.alerts.matchTodayStartingXi"),
      tab: "Squad",
      severity: "warn",
    });
  }

  return alerts;
}
