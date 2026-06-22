import {
  Radar,
  RadarChart,
  PolarGrid,
  PolarAngleAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { useChartTheme } from "../ui/charts/chartTheme";
import { ChartContainer } from "../ui/charts/ChartContainer";
import type { TeamStatsOverview } from "./TeamProfile.types";

interface TeamStyleRadarChartProps {
  overview: TeamStatsOverview;
  labels: {
    possession: string;
    shots: string;
    passes: string;
    tackles: string;
    interceptions: string;
    fouls: string;
  };
}

function clamp(v: number, min: number, max: number) {
  return Math.max(0, Math.min(100, ((v - min) / (max - min)) * 100));
}

export function TeamStyleRadarChart({
  overview,
  labels,
}: TeamStyleRadarChartProps) {
  const theme = useChartTheme();

  if (overview.matchesPlayed === 0) {
    return <ChartContainer isEmpty height={220} />;
  }

  const data = [
    {
      attr: labels.possession,
      value: Math.round(clamp(overview.possessionAverage ?? 50, 30, 70)),
      raw: `${(overview.possessionAverage ?? 0).toFixed(1)}%`,
    },
    {
      attr: labels.shots,
      value: Math.round(clamp(overview.metrics.shots.perMatch ?? 0, 3, 20)),
      raw: (overview.metrics.shots.perMatch ?? 0).toFixed(1),
    },
    {
      attr: labels.passes,
      value: Math.round(clamp(overview.metrics.passes.accuracy ?? 0, 50, 90)),
      raw: `${(overview.metrics.passes.accuracy ?? 0).toFixed(1)}%`,
    },
    {
      attr: labels.tackles,
      value: Math.round(clamp(overview.metrics.tacklesWon.perMatch ?? 0, 3, 25)),
      raw: (overview.metrics.tacklesWon.perMatch ?? 0).toFixed(1),
    },
    {
      attr: labels.interceptions,
      value: Math.round(clamp(overview.metrics.interceptions.perMatch ?? 0, 2, 15)),
      raw: (overview.metrics.interceptions.perMatch ?? 0).toFixed(1),
    },
    {
      attr: labels.fouls,
      value: Math.round(100 - clamp(overview.metrics.foulsCommitted.perMatch ?? 0, 5, 25)),
      raw: (overview.metrics.foulsCommitted.perMatch ?? 0).toFixed(1),
    },
  ];

  return (
    <ChartContainer height={220}>
      <ResponsiveContainer width="100%" height="100%">
        <RadarChart data={data} margin={{ top: 8, right: 28, bottom: 8, left: 28 }}>
          <PolarGrid stroke={theme.gridColor} />
          <PolarAngleAxis
            dataKey="attr"
            tick={{ fill: theme.axisColor, fontSize: 10, fontFamily: "var(--font-heading)" }}
          />
          <Radar
            dataKey="value"
            stroke={theme.primary}
            fill={theme.primary}
            fillOpacity={0.2}
            dot={{ fill: theme.primary, r: 3 }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: theme.tooltipBg,
              border: `1px solid ${theme.tooltipBorder}`,
              borderRadius: 8,
              fontSize: 11,
              color: theme.tooltipText,
            }}
            formatter={(_value, _name, props: { payload?: { raw?: string } }) => [
              props.payload?.raw ?? (_value ?? ""),
              "",
            ]}
          />
        </RadarChart>
      </ResponsiveContainer>
    </ChartContainer>
  );
}
