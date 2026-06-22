import {
  ComposedChart,
  Bar,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { useChartTheme } from "../ui/charts/chartTheme";
import { ChartContainer } from "../ui/charts/ChartContainer";
import type { TeamData } from "../../store/gameStore";

interface TeamSeasonHistoryChartProps {
  history: TeamData["history"];
  wonLabel: string;
  drawnLabel: string;
  lostLabel: string;
  positionLabel: string;
}

export function TeamSeasonHistoryChart({
  history,
  wonLabel,
  drawnLabel,
  lostLabel,
  positionLabel,
}: TeamSeasonHistoryChartProps) {
  const theme = useChartTheme();

  if (history.length === 0) {
    return <ChartContainer isEmpty height={180} />;
  }

  const data = history.map((r) => ({
    season: `${r.season}/${String(r.season + 1).slice(-2)}`,
    won: r.won,
    drawn: r.drawn,
    lost: r.lost,
    pos: r.league_position,
  }));

  const maxPlayed = Math.max(...history.map((r) => r.played), 10);
  const maxPos = Math.max(...history.map((r) => r.league_position), 5);

  return (
    <ChartContainer height={180}>
      <ResponsiveContainer width="100%" height="100%">
        <ComposedChart data={data} margin={{ top: 8, right: 40, bottom: 0, left: -8 }}>
          <CartesianGrid strokeDasharray="3 3" stroke={theme.gridColor} vertical={false} />
          <XAxis
            dataKey="season"
            tick={{ fill: theme.axisColor, fontSize: 10, fontFamily: "var(--font-heading)" }}
            axisLine={{ stroke: theme.gridColor }}
            tickLine={false}
          />
          <YAxis
            yAxisId="left"
            domain={[0, maxPlayed]}
            tick={{ fill: theme.axisColor, fontSize: 10 }}
            axisLine={false}
            tickLine={false}
            width={24}
          />
          <YAxis
            yAxisId="right"
            orientation="right"
            domain={[1, maxPos + 2]}
            reversed
            tick={{ fill: theme.axisColor, fontSize: 10 }}
            axisLine={false}
            tickLine={false}
            width={24}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: theme.tooltipBg,
              border: `1px solid ${theme.tooltipBorder}`,
              borderRadius: 8,
              fontSize: 11,
              color: theme.tooltipText,
            }}
          />
          <Legend
            wrapperStyle={{ fontSize: 10, paddingTop: 4, fontFamily: "var(--font-heading)" }}
          />
          <Bar yAxisId="left" dataKey="won" stackId="a" fill={theme.success} name={wonLabel} radius={[0, 0, 0, 0]} />
          <Bar yAxisId="left" dataKey="drawn" stackId="a" fill={theme.axisColor} name={drawnLabel} />
          <Bar yAxisId="left" dataKey="lost" stackId="a" fill={theme.danger} name={lostLabel} radius={[2, 2, 0, 0]} />
          <Line
            yAxisId="right"
            type="monotone"
            dataKey="pos"
            stroke={theme.secondary}
            strokeWidth={2}
            dot={{ fill: theme.secondary, r: 3 }}
            name={positionLabel}
          />
        </ComposedChart>
      </ResponsiveContainer>
    </ChartContainer>
  );
}
