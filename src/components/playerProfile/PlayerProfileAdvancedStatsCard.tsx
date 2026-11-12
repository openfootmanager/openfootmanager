import { useTranslation } from "react-i18next";
import { competitionDisplayName } from "../../lib/competitionName";
import { formatOrdinal } from "../../lib/ordinal";
import { Card, CardBody, CardHeader, ProgressBar } from "../ui";
import type { PlayerAdvancedStatsSummary } from "./PlayerProfile.helpers";

type TranslateFn = (key: string, options?: Record<string, unknown>) => string;

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

/**
 * One stat: its name and total, a bar at the backend's percentile with its ordinal (a high
 * percentile always means he does well; the number is drawn as given, never recomputed or judged),
 * and its rate in words. A stat with no percentile has no bar and a dash.
 */
function AdvancedStatRow({
  label,
  rate,
  total,
  percentile,
  percentileLabel,
  percentileUnavailableLabel,
  barLabel,
}: {
  label: string;
  rate: string;
  total: string;
  percentile: number | null;
  percentileLabel: string;
  percentileUnavailableLabel: string;
  barLabel: string;
}) {
  const { t, i18n } = useTranslation();
  return (
    <li className="flex flex-col gap-1.5 py-3 first:pt-0 last:pb-0">
      <div className="flex items-baseline justify-between gap-3">
        <p className="min-w-0 text-body font-semibold text-fg">{label}</p>
        <p className="shrink-0 font-heading text-lead font-bold tabular-nums text-fg">{total}</p>
      </div>
      <div className="flex items-center gap-3">
        <div className="min-w-0 flex-1">
          {percentile === null ? null : (
            <ProgressBar value={percentile} variant="muted" size="sm" label={barLabel} />
          )}
        </div>
        <p
          title={percentile === null ? percentileUnavailableLabel : percentileLabel}
          className="w-12 shrink-0 text-right text-caption font-semibold tabular-nums text-fg-muted"
        >
          {percentile === null ? "-" : formatOrdinal(percentile, i18n.language, t)}
        </p>
      </div>
      <p className="text-caption tabular-nums text-fg-muted">{rate}</p>
    </li>
  );
}

export default function PlayerProfileAdvancedStatsCard({
  summary,
  t,
}: PlayerProfileAdvancedStatsCardProps) {
  const labels = {
    title: t("playerProfile.advancedStats"),
    shots: resolveLabel(t, "playerProfile.shots", "Shots"),
    shotsOnTarget: resolveLabel(t, "playerProfile.shotsOnTarget", "Shots On Target"),
    passes: resolveLabel(t, "playerProfile.passes", "Passes"),
    tacklesWon: resolveLabel(t, "playerProfile.tacklesWon", "Tackles Won"),
    interceptions: resolveLabel(t, "playerProfile.interceptions", "Interceptions"),
    foulsCommitted: resolveLabel(t, "playerProfile.foulsCommitted", "Fouls Committed"),
    percentile: resolveLabel(t, "playerProfile.percentile", "Percentile"),
    percentileUnavailable: resolveLabel(
      t,
      "playerProfile.percentileUnavailable",
      "Percentile unavailable",
    ),
  };

  // A rate that does not exist (no minutes yet) is a dash, never "0.00 per 90".
  const perNinety = (value: number | null) =>
    value === null ? "-" : t("playerProfile.per90Value", { value: formatRate(value) });

  const rows = [
    {
      id: "shots",
      label: labels.shots,
      primaryValue: String(summary.metrics.shots.total),
      rate: perNinety(summary.metrics.shots.per90),
      percentile: summary.metrics.shots.percentile,
    },
    {
      id: "shotsOnTarget",
      label: labels.shotsOnTarget,
      primaryValue: String(summary.metrics.shotsOnTarget.total),
      rate: perNinety(summary.metrics.shotsOnTarget.per90),
      percentile: summary.metrics.shotsOnTarget.percentile,
    },
    {
      id: "passes",
      label: labels.passes,
      primaryValue: `${summary.metrics.passes.completed} / ${summary.metrics.passes.attempted}`,
      rate:
        summary.metrics.passes.accuracy === null
          ? "-"
          : t("playerProfile.accurateValue", {
              value: formatPercentage(summary.metrics.passes.accuracy),
            }),
      percentile: summary.metrics.passes.percentile,
    },
    {
      id: "tacklesWon",
      label: labels.tacklesWon,
      primaryValue: String(summary.metrics.tacklesWon.total),
      rate: perNinety(summary.metrics.tacklesWon.per90),
      percentile: summary.metrics.tacklesWon.percentile,
    },
    {
      id: "interceptions",
      label: labels.interceptions,
      primaryValue: String(summary.metrics.interceptions.total),
      rate: perNinety(summary.metrics.interceptions.per90),
      percentile: summary.metrics.interceptions.percentile,
    },
    {
      id: "foulsCommitted",
      label: labels.foulsCommitted,
      primaryValue: String(summary.metrics.foulsCommitted.total),
      rate: perNinety(summary.metrics.foulsCommitted.per90),
      percentile: summary.metrics.foulsCommitted.percentile,
    },
  ];

  return (
    <Card>
      <CardHeader>{labels.title}</CardHeader>
      <CardBody>
        {summary.peerGroup && (
          <p className="mb-3 text-caption text-fg-muted">
            {t("playerProfile.percentileGroup", {
              position: t(`common.positions.${summary.peerGroup.position}`),
              league: competitionDisplayName(
                {
                  name: summary.peerGroup.competitionName,
                  name_key: summary.peerGroup.competitionNameKey,
                  country_id: summary.peerGroup.countryId,
                  division_tier: summary.peerGroup.divisionTier,
                },
                t,
              ),
            })}
          </p>
        )}
        <ul className="flex flex-col divide-y divide-line">
          {rows.map((row) => (
            <AdvancedStatRow
              key={row.id}
              label={row.label}
              rate={row.rate}
              total={row.primaryValue}
              percentile={row.percentile}
              percentileLabel={labels.percentile}
              percentileUnavailableLabel={labels.percentileUnavailable}
              barLabel={t("playerProfile.percentileBar", { stat: row.label })}
            />
          ))}
        </ul>
      </CardBody>
    </Card>
  );
}
