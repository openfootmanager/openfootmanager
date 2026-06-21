import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from "recharts";
import { useChartTheme } from "../ui/charts/chartTheme";

interface PossessionDonutProps {
  homePct: number;
  awayPct: number;
  homeTeamName: string;
  awayTeamName: string;
  homeColor: string;
  awayColor: string;
  label: string;
}

export function PossessionDonut({
  homePct,
  awayPct,
  homeTeamName,
  awayTeamName,
  homeColor,
  awayColor,
  label,
}: PossessionDonutProps) {
  const theme = useChartTheme();
  const total = homePct + awayPct;
  const normalizedHome = total > 0 ? (homePct / total) * 100 : 50;
  const roundedHome = Math.round(normalizedHome);
  const roundedAway = 100 - roundedHome;
  const data = [
    { name: homeTeamName, value: roundedHome },
    { name: awayTeamName, value: roundedAway },
  ];

  return (
    <div className="flex flex-col items-center gap-1">
      <p className="text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
        {label}
      </p>
      <div className="relative" style={{ width: 72, height: 72 }}>
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie
              data={data}
              cx="50%"
              cy="50%"
              innerRadius={22}
              outerRadius={32}
              dataKey="value"
              strokeWidth={0}
            >
              <Cell fill={homeColor} />
              <Cell fill={awayColor} />
            </Pie>
            <Tooltip
              contentStyle={{
                backgroundColor: theme.tooltipBg,
                border: `1px solid ${theme.tooltipBorder}`,
                borderRadius: 6,
                fontSize: 11,
                color: theme.tooltipText,
              }}
              formatter={(value, name) => [`${value ?? 0}%`, String(name ?? "")]}
            />
          </PieChart>
        </ResponsiveContainer>
        <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
          <span className="text-[10px] font-heading font-bold text-gray-700 dark:text-gray-300 tabular-nums">
            {roundedHome}%
          </span>
        </div>
      </div>
      <div className="flex gap-3 text-[9px] font-heading uppercase tracking-wider">
        <span style={{ color: homeColor }}>{roundedHome}%</span>
        <span style={{ color: awayColor }}>{roundedAway}%</span>
      </div>
    </div>
  );
}
