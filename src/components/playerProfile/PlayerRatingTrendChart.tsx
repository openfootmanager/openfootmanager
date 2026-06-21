import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ReferenceLine,
  ResponsiveContainer,
  Dot,
} from "recharts";
import { useChartTheme } from "../ui/charts/chartTheme";
import { ChartContainer } from "../ui/charts/ChartContainer";
import type { PlayerRecentMatchEntry } from "./PlayerProfileRecentMatchesCard";

interface PlayerRatingTrendChartProps {
  matches: PlayerRecentMatchEntry[];
  ratingLabel: string;
}

function getRatingColor(rating: number, primary: string): string {
  if (rating >= 8) return "#ffd60a";
  if (rating >= 7) return "#22c55e";
  if (rating >= 6) return primary;
  if (rating >= 5) return "#eab308";
  return "#ef4444";
}

export function PlayerRatingTrendChart({
  matches,
  ratingLabel,
}: PlayerRatingTrendChartProps) {
  const theme = useChartTheme();

  if (matches.length < 2) {
    return <ChartContainer isEmpty height={140} />;
  }

  const data = [...matches].reverse().map((m, i) => ({
    matchday: m.matchday > 0 ? `MD${m.matchday}` : `M${i + 1}`,
    rating: Number(m.rating.toFixed(1)),
    opponent: m.opponent_name,
    result: `${m.team_goals}-${m.opponent_goals}`,
  }));

  return (
    <ChartContainer height={140}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 8, right: 8, bottom: 0, left: -16 }}>
          <CartesianGrid strokeDasharray="3 3" stroke={theme.gridColor} vertical={false} />
          <XAxis
            dataKey="matchday"
            tick={{ fill: theme.axisColor, fontSize: 9, fontFamily: "var(--font-heading)" }}
            axisLine={{ stroke: theme.gridColor }}
            tickLine={false}
          />
          <YAxis
            domain={[0, 10]}
            ticks={[0, 2, 4, 6, 8, 10]}
            tick={{ fill: theme.axisColor, fontSize: 9 }}
            axisLine={false}
            tickLine={false}
          />
          <ReferenceLine y={6} stroke={theme.axisColor} strokeDasharray="4 4" strokeOpacity={0.5} />
          <Tooltip
            contentStyle={{
              backgroundColor: theme.tooltipBg,
              border: `1px solid ${theme.tooltipBorder}`,
              borderRadius: 8,
              fontSize: 11,
              color: theme.tooltipText,
            }}
            formatter={(value) => [typeof value === "number" ? value.toFixed(1) : String(value ?? ""), ratingLabel]}
            labelFormatter={(label, payload) => {
              const item = payload?.[0]?.payload;
              return item ? `${item.opponent} (${item.result})` : label;
            }}
          />
          <Line
            type="monotone"
            dataKey="rating"
            stroke={theme.primary}
            strokeWidth={2}
            dot={(props) => {
              const { cx, cy, payload } = props;
              const color = getRatingColor(payload.rating, theme.primary);
              return <Dot key={`dot-${cx}-${cy}`} cx={cx} cy={cy} r={4} fill={color} stroke={theme.tooltipBg} strokeWidth={1.5} />;
            }}
            activeDot={{ r: 5, fill: theme.primary }}
          />
        </LineChart>
      </ResponsiveContainer>
    </ChartContainer>
  );
}
