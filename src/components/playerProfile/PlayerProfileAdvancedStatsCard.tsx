import { Card, CardBody, CardHeader } from "../ui";
import PlayerProfileStatCard from "./PlayerProfileStatCard";
import type { PlayerAdvancedStatsSummary } from "./PlayerProfile.helpers";

type TranslateFn = (key: string) => string;

interface PlayerProfileAdvancedStatsCardProps {
    summary: PlayerAdvancedStatsSummary;
    t: TranslateFn;
}

function resolveLabel(t: TranslateFn, key: string, fallback: string): string {
    const translated = t(key);
    return translated === key ? fallback : translated;
}

function formatRate(value: number | null): string {
    if (value === null) {
        return "-";
    }

    return value.toFixed(2);
}

function formatPercentage(value: number | null): string {
    if (value === null) {
        return "-";
    }

    return Number.isInteger(value) ? `${value}%` : `${value.toFixed(1)}%`;
}

function formatOrdinal(value: number | null, unavailableLabel: string): string {
    if (value === null) {
        return unavailableLabel;
    }

    const mod100 = value % 100;
    if (mod100 >= 11 && mod100 <= 13) {
        return `${value}th`;
    }

    switch (value % 10) {
        case 1:
            return `${value}st`;
        case 2:
            return `${value}nd`;
        case 3:
            return `${value}rd`;
        default:
            return `${value}th`;
    }
}

function AdvancedStatCard({
    label,
    primaryValue,
    secondaryLabel,
    secondaryValue,
    percentile,
    percentileLabel,
    percentileUnavailableLabel,
}: {
    label: string;
    primaryValue: string;
    secondaryLabel: string;
    secondaryValue: string;
    percentile: number | null;
    percentileLabel: string;
    percentileUnavailableLabel: string;
}) {
    return (
        <PlayerProfileStatCard
            label={label}
            headerRight={
                <span
                    title={percentile === null ? percentileUnavailableLabel : percentileLabel}
                    className="font-heading font-bold text-sm tabular-nums text-gray-700 dark:text-gray-200"
                >
                    {formatOrdinal(percentile, "-")}
                </span>
            }
        >
            <div className="mt-auto flex items-baseline justify-between gap-3">
                <span className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100 tabular-nums">
                    {primaryValue}
                </span>
                <span className="text-right">
                    <span className="block text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                        {secondaryLabel}
                    </span>
                    <span className="block font-heading font-bold text-sm text-gray-700 dark:text-gray-200 tabular-nums">
                        {secondaryValue}
                    </span>
                </span>
            </div>
        </PlayerProfileStatCard>
    );
}

export default function PlayerProfileAdvancedStatsCard({
    summary,
    t,
}: PlayerProfileAdvancedStatsCardProps) {
    const labels = {
        title: t("playerProfile.advancedStats"),
        shots: resolveLabel(t, "playerProfile.shots", "Shots"),
        shotsOnTarget: resolveLabel(
            t,
            "playerProfile.shotsOnTarget",
            "Shots On Target",
        ),
        passes: resolveLabel(t, "playerProfile.passes", "Passes"),
        tacklesWon: resolveLabel(t, "playerProfile.tacklesWon", "Tackles Won"),
        interceptions: resolveLabel(
            t,
            "playerProfile.interceptions",
            "Interceptions",
        ),
        foulsCommitted: resolveLabel(
            t,
            "playerProfile.foulsCommitted",
            "Fouls Committed",
        ),
        per90: resolveLabel(t, "playerProfile.per90", "Per 90"),
        passAccuracy: resolveLabel(
            t,
            "playerProfile.passAccuracy",
            "Pass Accuracy",
        ),
        percentile: resolveLabel(t, "playerProfile.percentile", "Percentile"),
        percentileUnavailable: resolveLabel(
            t,
            "playerProfile.percentileUnavailable",
            "Percentile unavailable",
        ),
    };

    const rows = [
        {
            id: "shots",
            label: labels.shots,
            primaryValue: String(summary.metrics.shots.total),
            secondaryLabel: labels.per90,
            secondaryValue: formatRate(summary.metrics.shots.per90),
            percentile: summary.metrics.shots.percentile,
        },
        {
            id: "shotsOnTarget",
            label: labels.shotsOnTarget,
            primaryValue: String(summary.metrics.shotsOnTarget.total),
            secondaryLabel: labels.per90,
            secondaryValue: formatRate(summary.metrics.shotsOnTarget.per90),
            percentile: summary.metrics.shotsOnTarget.percentile,
        },
        {
            id: "passes",
            label: labels.passes,
            primaryValue: `${summary.metrics.passes.completed} / ${summary.metrics.passes.attempted}`,
            secondaryLabel: labels.passAccuracy,
            secondaryValue: formatPercentage(summary.metrics.passes.accuracy),
            percentile: summary.metrics.passes.percentile,
        },
        {
            id: "tacklesWon",
            label: labels.tacklesWon,
            primaryValue: String(summary.metrics.tacklesWon.total),
            secondaryLabel: labels.per90,
            secondaryValue: formatRate(summary.metrics.tacklesWon.per90),
            percentile: summary.metrics.tacklesWon.percentile,
        },
        {
            id: "interceptions",
            label: labels.interceptions,
            primaryValue: String(summary.metrics.interceptions.total),
            secondaryLabel: labels.per90,
            secondaryValue: formatRate(summary.metrics.interceptions.per90),
            percentile: summary.metrics.interceptions.percentile,
        },
        {
            id: "foulsCommitted",
            label: labels.foulsCommitted,
            primaryValue: String(summary.metrics.foulsCommitted.total),
            secondaryLabel: labels.per90,
            secondaryValue: formatRate(summary.metrics.foulsCommitted.per90),
            percentile: summary.metrics.foulsCommitted.percentile,
        },
    ];

    return (
        <Card>
            <CardHeader>{labels.title}</CardHeader>
            <CardBody>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 sm:auto-rows-fr">
                    {rows.map((row) => (
                        <AdvancedStatCard
                            key={row.id}
                            label={row.label}
                            primaryValue={row.primaryValue}
                            secondaryLabel={row.secondaryLabel}
                            secondaryValue={row.secondaryValue}
                            percentile={row.percentile}
                            percentileLabel={labels.percentile}
                            percentileUnavailableLabel={labels.percentileUnavailable}
                        />
                    ))}
                </div>
            </CardBody>
        </Card>
    );
}