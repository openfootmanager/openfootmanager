import type { GameStateData, MessageAction, MessageData } from "../../store/gameStore";
import { formatVal } from "../../lib/helpers";
import type { FinanceSnapshotData } from "../../services/financeService";

export type FacilityId = "Training" | "Medical" | "Scouting";

export interface FacilityUpgradeErrorState {
  facilityId: FacilityId;
  message: string;
}

export interface FacilityDefinition {
  effectKey: string;
  id: FacilityId;
  levelKey: "training" | "medical" | "scouting";
  titleKey: string;
}

export const DEFAULT_FACILITIES = {
  training: 1,
  medical: 1,
  scouting: 1,
};

export const FACILITY_DEFINITIONS: FacilityDefinition[] = [
  {
    id: "Training",
    levelKey: "training",
    titleKey: "finances.facilityTraining",
    effectKey: "finances.facilityTrainingEffect",
  },
  {
    id: "Medical",
    levelKey: "medical",
    titleKey: "finances.facilityMedical",
    effectKey: "finances.facilityMedicalEffect",
  },
  {
    id: "Scouting",
    levelKey: "scouting",
    titleKey: "finances.facilityScouting",
    effectKey: "finances.facilityScoutingEffect",
  },
];

export function formatSignedAmount(value: number): string {
  // formatVal already renders the sign (see prefixCurrency), so taking the
  // absolute value and re-attaching "-" here only duplicated that work.
  return formatVal(value);
}

/**
 * Identifies the club state a backend finance snapshot was computed from, so a snapshot fetched
 * for an older state is recognised as stale.
 */
export function buildFinanceSnapshotKey(
  gameState: GameStateData,
  myTeam: GameStateData["teams"][number],
): string {
  const financePlayers = gameState.players.filter(
    (player) =>
      player.team_id === myTeam.id ||
      player.active_loan?.parent_team_id === myTeam.id ||
      player.active_loan?.loan_team_id === myTeam.id,
  );
  const teamStaff = gameState.staff.filter((staffMember) => staffMember.team_id === myTeam.id);
  return [
    myTeam.id,
    gameState.clock.current_date,
    myTeam.finance,
    myTeam.wage_budget,
    myTeam.transfer_budget,
    myTeam.season_income,
    myTeam.season_expenses,
    myTeam.facilities?.training ?? DEFAULT_FACILITIES.training,
    myTeam.facilities?.medical ?? DEFAULT_FACILITIES.medical,
    myTeam.facilities?.scouting ?? DEFAULT_FACILITIES.scouting,
    myTeam.sponsorship?.sponsor_name ?? "",
    myTeam.sponsorship?.base_value ?? 0,
    myTeam.sponsorship?.remaining_weeks ?? 0,
    financePlayers
      .map((player) =>
        [
          player.id,
          player.team_id ?? "",
          player.wage,
          player.contract_end ?? "",
          player.active_loan?.parent_team_id ?? "",
          player.active_loan?.loan_team_id ?? "",
          player.active_loan?.wage_contribution_pct ?? "",
          player.active_loan?.end_date ?? "",
        ].join(":"),
      )
      .join("|"),
    teamStaff.map((staffMember) => `${staffMember.id}:${staffMember.wage}`).join("|"),
    gameState.messages
      .filter(isPendingSponsorOffer)
      .map((message) => message.id)
      .join("|"),
  ].join("::");
}

export interface ResolveMessageActionResult {
  game: GameStateData;
  effect: string | null;
  effect_i18n_key?: string | null;
  effect_i18n_params?: Record<string, string> | null;
}

export interface DelegatedRenewalResponseData {
  game: GameStateData;
  report: {
    success_count: number;
    failure_count: number;
    stalled_count: number;
  };
}

export interface TaggedFinanceSnapshotData {
  key: string;
  teamId: string;
  data: FinanceSnapshotData;
}

export interface BoardSupportResponseData {
  game: GameStateData;
  result: {
    support_amount: number;
    transfer_budget_reduction: number;
    satisfaction_penalty: number;
  };
}

export interface SponsorPitchResponseData {
  game: GameStateData;
  result: {
    message_id: string;
    sponsor_name: string;
    weekly_amount: number;
    duration_weeks: number;
  };
}

export interface MarketingCampaignResponseData {
  game: GameStateData;
  result: {
    message_id: string;
    gross_revenue: number;
    campaign_cost: number;
    net_income: number;
    cooldown_days: number;
  };
}

export function isChooseOptionAction(actionType: MessageAction["action_type"]): actionType is {
  ChooseOption: {
    options: Array<{ id: string; label: string; description: string }>;
  };
} {
  return typeof actionType === "object" && "ChooseOption" in actionType;
}

export function isPendingSponsorOffer(message: MessageData): boolean {
  return (
    message.id.startsWith("sponsor_") &&
    message.category === "Finance" &&
    message.actions.some((action) => !action.resolved && isChooseOptionAction(action.action_type))
  );
}
