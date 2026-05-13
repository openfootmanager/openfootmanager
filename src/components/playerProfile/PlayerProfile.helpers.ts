import type {
    PlayerData,
    PlayerSeasonStats,
    TeamData,
} from "../../store/gameStore";
import type { TOptions } from "i18next";
import { annualAmountToWeeklyCommitment } from "../../lib/finance";
import { formatExactMoney, formatVal, formatWeeklyAmount } from "../../lib/helpers";

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

export interface PlayerAdvancedStatsSummary {
    percentileEligible: boolean;
    metrics: {
        shots: PlayerAdvancedMetric;
        shotsOnTarget: PlayerAdvancedMetric;
        passes: PlayerAdvancedPassMetric;
        tacklesWon: PlayerAdvancedMetric;
        interceptions: PlayerAdvancedMetric;
        foulsCommitted: PlayerAdvancedMetric;
    };
}

interface BuildPlayerAdvancedStatsOptions {
    minimumMinutes?: number;
    minimumCohortSize?: number;
}

const DEFAULT_MINIMUM_MINUTES = 180;
const DEFAULT_MINIMUM_COHORT_SIZE = 3;

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

export function getPlayerAge(
    dateOfBirth: string,
    asOfDate: string = "2026-07-01",
): number {
    const birthDate = new Date(dateOfBirth);
    const currentDate = new Date(asOfDate);
    let age = currentDate.getFullYear() - birthDate.getFullYear();

    if (
        currentDate.getMonth() < birthDate.getMonth() ||
        (currentDate.getMonth() === birthDate.getMonth() &&
            currentDate.getDate() < birthDate.getDate())
    ) {
        age -= 1;
    }

    return age;
}

export function formatPlayerMarketValue(value: number): string {
    return formatVal(value);
}

export function formatPlayerWage(
    annualWage: number,
    weeklySuffix: string,
): string {
    const weeklyWage = annualAmountToWeeklyCommitment(annualWage);
    return formatWeeklyAmount(formatExactMoney(weeklyWage), weeklySuffix);
}

export function getAttributeColorClass(value: number): string {
    if (value >= 80) {
        return "text-primary-500 dark:text-primary-400";
    }
    if (value >= 60) {
        return "text-accent-600 dark:text-accent-400";
    }
    if (value >= 40) {
        return "text-gray-600 dark:text-gray-400";
    }
    return "text-red-500 dark:text-red-400";
}

export function resolvePlayerInjuryName(
    injuryName: string,
    translate: TranslateFn,
): string {
    if (injuryName.includes(".")) {
        return translate(injuryName, { defaultValue: injuryName });
    }

    return translate(`common.injuries.${injuryName}`, {
        defaultValue: injuryName,
    });
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

function positionKey(player: PlayerData): string {
    return player.natural_position || player.position;
}

function percentileRank(values: number[], target: number | null): number | null {
    if (target === null || values.length === 0) {
        return null;
    }

    const rankedCount = values.filter((value) => value <= target).length;
    return Math.round((rankedCount / values.length) * 100);
}

function eligiblePeers(
    player: PlayerData,
    players: PlayerData[],
    minimumMinutes: number,
): PlayerData[] {
    const targetPosition = positionKey(player);

    return players.filter((candidate) => {
        return (
            positionKey(candidate) === targetPosition &&
            statValue(candidate.stats.minutes_played) >= minimumMinutes
        );
    });
}

function metricPercentile(
    peers: PlayerData[],
    selector: (stats: PlayerSeasonStats) => number | null,
    playerStats: PlayerSeasonStats,
): number | null {
    const peerValues = peers
        .map((candidate) => selector(candidate.stats))
        .filter((value): value is number => value !== null);

    return percentileRank(peerValues, selector(playerStats));
}

export function buildPlayerAdvancedStats(
    player: PlayerData,
    players: PlayerData[],
    options: BuildPlayerAdvancedStatsOptions = {},
): PlayerAdvancedStatsSummary {
    const minimumMinutes = options.minimumMinutes ?? DEFAULT_MINIMUM_MINUTES;
    const minimumCohortSize =
        options.minimumCohortSize ?? DEFAULT_MINIMUM_COHORT_SIZE;
    const minutesPlayed = statValue(player.stats.minutes_played);
    const percentileEligible = minutesPlayed >= minimumMinutes;
    const peers = eligiblePeers(player, players, minimumMinutes);
    const canComputePercentiles =
        percentileEligible && peers.length >= minimumCohortSize;

    const shots = statValue(player.stats.shots);
    const shotsOnTarget = statValue(player.stats.shots_on_target);
    const passesCompleted = statValue(player.stats.passes_completed);
    const passesAttempted = statValue(player.stats.passes_attempted);
    const tacklesWon = statValue(player.stats.tackles_won);
    const interceptions = statValue(player.stats.interceptions);
    const foulsCommitted = statValue(player.stats.fouls_committed);

    return {
        percentileEligible: canComputePercentiles,
        metrics: {
            shots: {
                total: shots,
                per90: calculatePer90(shots, minutesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePer90(
                                statValue(stats.shots),
                                statValue(stats.minutes_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            shotsOnTarget: {
                total: shotsOnTarget,
                per90: calculatePer90(shotsOnTarget, minutesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePer90(
                                statValue(stats.shots_on_target),
                                statValue(stats.minutes_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            passes: {
                completed: passesCompleted,
                attempted: passesAttempted,
                accuracy: calculatePassAccuracy(passesCompleted, passesAttempted),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePassAccuracy(
                                statValue(stats.passes_completed),
                                statValue(stats.passes_attempted),
                            ),
                        player.stats,
                    )
                    : null,
            },
            tacklesWon: {
                total: tacklesWon,
                per90: calculatePer90(tacklesWon, minutesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePer90(
                                statValue(stats.tackles_won),
                                statValue(stats.minutes_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            interceptions: {
                total: interceptions,
                per90: calculatePer90(interceptions, minutesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePer90(
                                statValue(stats.interceptions),
                                statValue(stats.minutes_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            foulsCommitted: {
                total: foulsCommitted,
                per90: calculatePer90(foulsCommitted, minutesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePer90(
                                statValue(stats.fouls_committed),
                                statValue(stats.minutes_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
        },
    };
}

