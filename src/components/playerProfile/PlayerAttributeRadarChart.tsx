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
import type { PlayerAttributeGroup } from "./PlayerProfile.attributes";

interface PlayerAttributeRadarChartProps {
  attrGroups: PlayerAttributeGroup[];
  isGk: boolean;
}

interface RadarEntry {
  attr: string;
  value: number;
  fullMark: number;
}

function pickOutfieldAttrs(groups: PlayerAttributeGroup[]): RadarEntry[] {
  // Build by picking from groups[0]=physical, groups[1]=technical, groups[2]=mental
  const physGroup = groups[0]?.attrs ?? [];
  const techGroup = groups[1]?.attrs ?? [];
  const mentGroup = groups[2]?.attrs ?? [];

  const selected = [
    physGroup[0], // pace
    techGroup[1], // shooting
    techGroup[0], // passing
    techGroup[3], // dribbling
    techGroup[2], // tackling
    mentGroup[1], // vision
    mentGroup[5], // teamwork
    physGroup[1], // stamina
  ].filter(Boolean) as { name: string; value: number }[];

  return selected.map((a) => ({ attr: a.name, value: a.value, fullMark: 100 }));
}

function pickGkAttrs(groups: PlayerAttributeGroup[]): RadarEntry[] {
  const gkGroup = groups.find((_, i) => i === 3)?.attrs ?? groups[groups.length - 1]?.attrs ?? [];
  const mentGroup = groups[2]?.attrs ?? [];
  const physGroup = groups[0]?.attrs ?? [];

  const selected = [
    gkGroup[0], // handling
    gkGroup[1], // reflexes
    gkGroup[2], // aerial
    mentGroup[3], // composure
    mentGroup[1], // vision
    mentGroup[5], // teamwork
    physGroup[1], // stamina
    physGroup[2], // strength
  ].filter(Boolean) as { name: string; value: number }[];

  return selected.map((a) => ({ attr: a.name, value: a.value, fullMark: 100 }));
}

export function PlayerAttributeRadarChart({
  attrGroups,
  isGk,
}: PlayerAttributeRadarChartProps) {
  const theme = useChartTheme();

  if (attrGroups.length === 0) {
    return <ChartContainer isEmpty height={240} />;
  }

  const data = isGk ? pickGkAttrs(attrGroups) : pickOutfieldAttrs(attrGroups);

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
