import { formatDate } from "../../lib/dateFormatting";
import type { PlayerData, TeamData } from "../../store/gameStore";
import type { TOptions } from "i18next";
import type { CompetitionSeasonPhase } from "../../store/generated/CompetitionSeasonPhase";
import { weeklyWageAmount } from "../../lib/finance";
import { formatVal, formatWeeklyWage, getContractYearsRemaining } from "../../lib/helpers";
import { getAttributeValueClassName } from "../../lib/playerAttributeDisplay";
import { resolveInjuryName } from "../../lib/injury";

type TranslateFn = (key: string, options?: TOptions) => string;

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
  divisionTier?: number | null;
  divisionPhase?: CompetitionSeasonPhase | null;
  /** A `Position` name such as `Striker`. */
  position: string;
}

/** One of a player's recent matches, latest first, as the backend returns it. */
export interface PlayerRecentMatchEntry {
  fixture_id: string;
  date: string;
  competition: string;
  matchday: number;
  opponent_team_id: string;
  opponent_name: string;
  team_goals: number;
  opponent_goals: number;
  minutes_played: number;
  goals: number;
  assists: number;
  shots: number;
  shots_on_target: number;
  rating: number;
  rated: boolean;
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

/**
 * A day from the ledger or a contract, written the way the player's language writes dates
 * (`formatDate`, the project's one formatter): `Jun 30, 2033`. A value that is not a day (an
 * unknown date, empty) is returned as it came, never as today.
 */
export function formatProfileDay(day: string, language: string): string {
  return formatDate(day, language, { day: "numeric", month: "short", year: "numeric" });
}

type YearsLeftTranslate = (key: string, options?: Record<string, string | number>) => string;

/**
 * The years left on a contract as the sheet writes them: "1.3 years", one decimal, the number in
 * the language's own decimal separator and the noun in the form that language gives a fraction.
 *
 * The form comes from CLDR through `Intl.PluralRules`, asked about the number as it is shown (one
 * decimal), because that is what decides it: French and Brazilian Portuguese keep the singular for
 * 1.3, Czech has its own form for a fraction, English and Russian use the plural. The repo has no
 * i18next plural machinery yet (#467), so the three forms a language can need here are three
 * plain keys; the number goes in as `years`, never `count`, so i18next does not go looking for
 * `_one`/`_other` variants.
 */
export function formatYearsLeft(
  contractEnd: string,
  currentDate: string,
  language: string,
  t: YearsLeftTranslate,
): string {
  const years = Number(getContractYearsRemaining(contractEnd, currentDate));
  const oneDecimal = { minimumFractionDigits: 1, maximumFractionDigits: 1 };
  const category = new Intl.PluralRules(language, oneDecimal).select(years);
  const form = category === "one" || category === "many" ? category : "other";
  return t(`playerProfile.contractSheet.yearsLeft.${form}`, {
    years: new Intl.NumberFormat(language, oneDecimal).format(years),
  });
}
