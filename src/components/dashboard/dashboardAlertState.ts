import { formatVal } from "../../lib/helpers";
import { getTeamFinanceSnapshot } from "../../lib/finance";
import type { GameStateData } from "../../store/gameStore";
import { buildStartingXIIds } from "../squad/SquadTab.helpers";

export interface DashboardAlert {
  id: string;
  text: string;
  tab: string;
  severity: "warn" | "info";
}

type DashboardAlertTranslator = (
  key: string,
  options?: Record<string, unknown>,
) => string;

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
          defaultValue:
            "Finances critical — balance {{balance}}, runway {{weeks}} week(s)",
        }),
        tab: "Finances",
        severity: "warn",
      });
    } else if (financeSnapshot.runwayStatus === "warning") {
      alerts.push({
        id: "finance_runway",
        text: t("dashboard.alerts.financeRunway", {
          count: financeSnapshot.cashRunwayWeeks,
          defaultValue: "Cash runway down to {{count}} week(s)",
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
          defaultValue: "Wage bill at {{percent}}% of budget",
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