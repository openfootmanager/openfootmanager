import { useTranslation } from "react-i18next";
import {
  Radar,
  RadarChart,
  PolarGrid,
  PolarAngleAxis,
  ResponsiveContainer,
  Tooltip,
} from "recharts";
import { useChartTheme } from "../ui/charts/chartTheme";
import { ChartContainer } from "../ui/charts/ChartContainer";
import type { PlayerData } from "../../store/gameStore";
import {
  getPlayerAttributeEntry,
  type PlayerAttributeKey,
} from "./PlayerProfile.attributes";

interface PlayerAttributeRadarChartProps {
  player: PlayerData;
  isGk: boolean;
}

interface RadarEntry {
  attr: string;
  value: number;
  fullMark: number;
}

// Fixed radar picks per player type — kept as attribute keys, not positional
// indices, so reordering the profile's attribute groups can't silently change
// what the radar shows.
const OUTFIELD_RADAR_KEYS: readonly PlayerAttributeKey[] = [
  "pace",
  "shooting",
  "passing",
  "dribbling",
  "tackling",
  "vision",
  "teamwork",
  "stamina",
];

const GK_RADAR_KEYS: readonly PlayerAttributeKey[] = [
  "handling",
  "reflexes",
  "aerial",
  "composure",
  "vision",
  "teamwork",
  "stamina",
  "strength",
];

function buildRadarData(
  player: PlayerData,
  keys: readonly PlayerAttributeKey[],
  translate: (key: string) => string,
): RadarEntry[] {
  return keys.map((key) => {
    const entry = getPlayerAttributeEntry(player, key, translate);
    return { attr: entry.name, value: entry.value, fullMark: 100 };
  });
}

export function PlayerAttributeRadarChart({
  player,
  isGk,
}: PlayerAttributeRadarChartProps) {
  const { t } = useTranslation();
  const theme = useChartTheme();

  const data = buildRadarData(player, isGk ? GK_RADAR_KEYS : OUTFIELD_RADAR_KEYS, t);

  if (data.length === 0) {
    return <ChartContainer isEmpty height={240} />;
  }

  return (
    <ChartContainer height={240}>
      <ResponsiveContainer width="100%" height="100%">
        <RadarChart data={data} margin={{ top: 8, right: 24, bottom: 8, left: 24 }}>
          <PolarGrid stroke={theme.gridColor} />
          <PolarAngleAxis
            dataKey="attr"
            tick={{ fill: theme.axisColor, fontSize: 10, fontFamily: "var(--font-heading)" }}
          />
          <Radar
            dataKey="value"
            stroke={theme.primary}
            fill={theme.primary}
            fillOpacity={0.25}
            dot={{ fill: theme.primary, r: 2 }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: theme.tooltipBg,
              border: `1px solid ${theme.tooltipBorder}`,
              borderRadius: 8,
              fontSize: 12,
              color: theme.tooltipText,
            }}
            formatter={(value) => [value ?? ""]}
          />
        </RadarChart>
      </ResponsiveContainer>
    </ChartContainer>
  );
}
