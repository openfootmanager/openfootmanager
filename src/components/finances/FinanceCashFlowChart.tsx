import { useId } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { useChartTheme } from "../ui/charts/chartTheme";
import { ChartContainer } from "../ui/charts/ChartContainer";
import { useTranslation } from "react-i18next";
import { formatDateShort, formatPreciseCompact, formatVal } from "../../lib/helpers";
import type { WeeklyCashFlowData } from "../../services/financeService";

interface FinanceCashFlowChartProps {
  /** The backend's weeks of cash flow (from its cash journal), drawn as they are: nothing is summed here. */
  weeks: WeeklyCashFlowData[];
  incomeLabel: string;
  expensesLabel: string;
}

export function FinanceCashFlowChart({
  weeks,
  incomeLabel,
  expensesLabel,
}: FinanceCashFlowChartProps) {
  const id = useId();
  const theme = useChartTheme();
  const { i18n } = useTranslation();
  // Each week is labelled by the Monday it starts on, in the player's language.
  const data = weeks.map((week) => ({
    week: formatDateShort(week.weekStart, i18n.language),
    income: week.income,
    expenses: week.expenses,
  }));
  const incomeGradId = `incomeGrad-${id}`;
  const expensesGradId = `expensesGrad-${id}`;

  if (data.length === 0) {
    return <ChartContainer isEmpty height={180} />;
  }

  return (
    <ChartContainer height={180}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data} margin={{ top: 8, right: 8, bottom: 0, left: 0 }}>
          <defs>
            <linearGradient id={incomeGradId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={theme.primary} stopOpacity={0.3} />
              <stop offset="95%" stopColor={theme.primary} stopOpacity={0.02} />
            </linearGradient>
            <linearGradient id={expensesGradId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={theme.danger} stopOpacity={0.3} />
              <stop offset="95%" stopColor={theme.danger} stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke={theme.gridColor} vertical={false} />
          <XAxis
            dataKey="week"
            tick={{ fill: theme.axisColor, fontSize: 9, fontFamily: "var(--font-heading)" }}
            axisLine={{ stroke: theme.gridColor }}
            tickLine={false}
          />
          <YAxis
            tickFormatter={formatVal}
            tick={{ fill: theme.axisColor, fontSize: 9 }}
            axisLine={false}
            tickLine={false}
            width="auto"
          />
          <Tooltip
            contentStyle={{
              backgroundColor: theme.tooltipBg,
              border: `1px solid ${theme.tooltipBorder}`,
              borderRadius: 8,
              fontSize: 11,
              color: theme.tooltipText,
            }}
            formatter={(value, name) => [
              typeof value === "number" ? formatPreciseCompact(value) : String(value ?? ""),
              String(name ?? ""),
            ]}
          />
          <Legend wrapperStyle={{ fontSize: 10, fontFamily: "var(--font-heading)" }} />
          <Area
            type="monotone"
            dataKey="income"
            name={incomeLabel}
            stroke={theme.primary}
            fill={`url(#${incomeGradId})`}
            strokeWidth={2}
          />
          <Area
            type="monotone"
            dataKey="expenses"
            name={expensesLabel}
            stroke={theme.danger}
            fill={`url(#${expensesGradId})`}
            strokeWidth={2}
          />
        </AreaChart>
      </ResponsiveContainer>
    </ChartContainer>
  );
}
