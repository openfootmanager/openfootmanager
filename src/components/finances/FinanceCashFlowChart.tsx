import { useId } from "react";
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from "recharts";
import { useChartTheme } from "../ui/charts/chartTheme";
import { ChartContainer } from "../ui/charts/ChartContainer";
import type { FinancialTransactionData } from "../../store/types";

interface FinanceCashFlowChartProps {
  ledger: FinancialTransactionData[];
  incomeLabel: string;
  expensesLabel: string;
}

function formatShortAmount(value: number): string {
  const abs = Math.abs(value);
  const sign = value < 0 ? "-" : "";
  if (abs >= 1_000_000) return `${sign}${(abs / 1_000_000).toFixed(1)}M`;
  if (abs >= 1_000) return `${sign}${(abs / 1_000).toFixed(0)}K`;
  return `${sign}${abs}`;
}

interface WeeklyBucket {
  week: string;
  income: number;
  expenses: number;
}

function aggregateByWeek(ledger: FinancialTransactionData[]): WeeklyBucket[] {
  if (ledger.length === 0) return [];

  const buckets: Record<string, WeeklyBucket> = {};

  for (const tx of ledger) {
    const d = new Date(tx.date);
    const year = d.getUTCFullYear();
    const jan1 = Date.UTC(year, 0, 1);
    const dayOfYear = Math.floor((d.getTime() - jan1) / 86400000);
    const weekNum = Math.floor(dayOfYear / 7) + 1;
    const key = `${year}-W${String(weekNum).padStart(2, "0")}`;
    const label = `W${weekNum}`;

    if (!buckets[key]) {
      buckets[key] = { week: label, income: 0, expenses: 0 };
    }

    if (tx.amount >= 0) {
      buckets[key].income += tx.amount;
    } else {
      buckets[key].expenses += Math.abs(tx.amount);
    }
  }

  return Object.keys(buckets)
    .sort()
    .slice(-12)
    .map((k) => buckets[k]);
}

export function FinanceCashFlowChart({
  ledger,
  incomeLabel,
  expensesLabel,
}: FinanceCashFlowChartProps) {
  const id = useId();
  const theme = useChartTheme();
  const data = aggregateByWeek(ledger);
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
            tickFormatter={formatShortAmount}
            tick={{ fill: theme.axisColor, fontSize: 9 }}
            axisLine={false}
            tickLine={false}
            width={36}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: theme.tooltipBg,
              border: `1px solid ${theme.tooltipBorder}`,
              borderRadius: 8,
              fontSize: 11,
              color: theme.tooltipText,
            }}
            formatter={(value, name) => [typeof value === "number" ? formatShortAmount(value) : String(value ?? ""), String(name ?? "")]}
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
