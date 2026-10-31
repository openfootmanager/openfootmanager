import type { PlayerData, TeamData } from "../../store/gameStore";
import type { TOptions } from "i18next";
import { weeklyWageAmount } from "../../lib/finance";
import { formatVal, formatWeeklyWage } from "../../lib/helpers";
import { getAttributeValueClassName } from "../../lib/playerAttributeDisplay";
import { resolveInjuryName } from "../../lib/injury";
import type { VisibleWage } from "../../services/scoutingService";

type TranslateFn = (key: string, options?: TOptions) => string;

/** A wage the club may see, written with `format`: as it is, as "about …", or "–". */
export function visibleWageText(
  wage: VisibleWage,
  format: (weeklyWage: number) => string,
  t: TranslateFn,
): string {
  if (wage.kind === "exact") return format(wage.weekly);
  if (wage.kind === "approximate") {
    return t("playerProfile.approximateWage", { wage: format(wage.weekly) });
  }
  return "–";
}

interface PlayerAdvancedMetric {
  total: number;
  per90: number | null;
  percentile: number | null;
}

interface PlayerAdvancedPassMetric {
  completed: number;
  attempted: number;
  accuracy: number | null;
  percentile: number | null;
}

/** Who the percentiles compare the player with; the backend names it. */
export interface PercentilePeerGroup {
  competitionId: string;
  competitionName: string;
  competitionNameKey: string | null;
  countryId: string | null;
  /** A `Position` name such as `Striker`. */
  position: string;
}

export interface PlayerAdvancedStatsSummary {
  percentileEligible: boolean;
  peerGroup: PercentilePeerGroup | null;
  /** Decided by the backend; the squad-data fallback cannot know it. */
  seasonRatingRated: boolean;
  metrics: {
    shots: PlayerAdvancedMetric;
    shotsOnTarget: PlayerAdvancedMetric;
    passes: PlayerAdvancedPassMetric;
    tacklesWon: PlayerAdvancedMetric;
    interceptions: PlayerAdvancedMetric;
    foulsCommitted: PlayerAdvancedMetric;
  };
}

export function getPlayerTeamName(
  teams: TeamData[],
  teamId: string | null,
  labels: {
    freeAgent: string;
    unknown: string;
  },
): string {
  if (!teamId) {
    return labels.freeAgent;
  }

  return teams.find((team) => team.id === teamId)?.name ?? labels.unknown;
}

export function formatPlayerMarketValue(value: number): string {
  return formatVal(value);
}

export function formatPlayerWage(weeklyWage: number): string {
  return formatWeeklyWage(weeklyWageAmount(weeklyWage));
}

/** The compact form for the profile's header tiles. */
export function formatPlayerWageCompact(weeklyWage: number): string {
  return formatWeeklyWage(weeklyWage, "compact");
}

export function getAttributeColorClass(value: number): string {
  return getAttributeValueClassName(value);
}

export function resolvePlayerInjuryName(injuryName: string, translate: TranslateFn): string {
  return resolveInjuryName(injuryName, translate);
}

function statValue(value: number | undefined): number {
  return value ?? 0;
}

function roundTo(value: number, digits: number): number {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

function calculatePer90(total: number, minutesPlayed: number): number | null {
  if (minutesPlayed <= 0) {
    return null;
  }

  return roundTo((total * 90) / minutesPlayed, 1);
}

function calculatePassAccuracy(completed: number, attempted: number): number | null {
  if (attempted <= 0) {
    return null;
  }

  return roundTo((completed / attempted) * 100, 1);
}

/**
 * What the squad data alone can say before the backend's overview arrives: totals and per-90
 * rates, no percentiles. Percentiles need a league peer group and each stat's direction, which
 * only the backend defines, so none is guessed here.
 */
export function buildPlayerAdvancedStats(player: PlayerData): PlayerAdvancedStatsSummary {
  const minutesPlayed = statValue(player.stats.minutes_played);
  const shots = statValue(player.stats.shots);
  const shotsOnTarget = statValue(player.stats.shots_on_target);
  const passesCompleted = statValue(player.stats.passes_completed);
  const passesAttempted = statValue(player.stats.passes_attempted);
  const tacklesWon = statValue(player.stats.tackles_won);
  const interceptions = statValue(player.stats.interceptions);
  const foulsCommitted = statValue(player.stats.fouls_committed);

  return {
    percentileEligible: false,
    peerGroup: null,
    seasonRatingRated: false,
    metrics: {
      shots: { total: shots, per90: calculatePer90(shots, minutesPlayed), percentile: null },
      shotsOnTarget: {
        total: shotsOnTarget,
        per90: calculatePer90(shotsOnTarget, minutesPlayed),
        percentile: null,
      },
      passes: {
        completed: passesCompleted,
        attempted: passesAttempted,
        accuracy: calculatePassAccuracy(passesCompleted, passesAttempted),
        percentile: null,
      },
      tacklesWon: {
        total: tacklesWon,
        per90: calculatePer90(tacklesWon, minutesPlayed),
        percentile: null,
      },
      interceptions: {
        total: interceptions,
        per90: calculatePer90(interceptions, minutesPlayed),
        percentile: null,
      },
      foulsCommitted: {
        total: foulsCommitted,
        per90: calculatePer90(foulsCommitted, minutesPlayed),
        percentile: null,
      },
    },
  };
}
