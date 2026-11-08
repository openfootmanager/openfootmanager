import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import { formatDateShort, formatVal } from "../../lib/helpers";
import { formatPreciseCompact } from "../../lib/valueFormatting";
import type { WeeklyCashFlowData } from "../../services/financeService";
import { FinanceCashFlowChart } from "./FinanceCashFlowChart";

// What the chart hands recharts for its axis and tooltip, captured: jsdom has no layout, so the
// real charts draw nothing to read the labels from.
const handedToCharts: {
  data?: unknown;
  tickFormatter?: (value: number) => string;
  tooltipFormatter?: (value: unknown, name: unknown) => [string, string];
  yAxisWidth?: number | "auto";
} = {};

vi.mock("recharts", () => {
  const Passthrough = ({ children }: { children?: ReactNode }) => <div>{children}</div>;
  return {
    ResponsiveContainer: Passthrough,
    AreaChart: (props: { data: unknown; children?: ReactNode }) => {
      handedToCharts.data = props.data;
      return <div>{props.children}</div>;
    },
    Area: () => null,
    XAxis: () => null,
    CartesianGrid: () => null,
    Legend: () => null,
    YAxis: (props: { tickFormatter: (value: number) => string; width: number | "auto" }) => {
      handedToCharts.tickFormatter = props.tickFormatter;
      handedToCharts.yAxisWidth = props.width;
      return null;
    },
    Tooltip: (props: { formatter: (value: unknown, name: unknown) => [string, string] }) => {
      handedToCharts.tooltipFormatter = props.formatter;
      return null;
    },
  };
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "de" } }),
}));

const weeks: WeeklyCashFlowData[] = [
  { weekStart: "2026-08-03", income: 1_250_000, expenses: 980_000 },
  { weekStart: "2026-08-10", income: 310_000, expenses: 40_000 },
];

describe("FinanceCashFlowChart", () => {
  /**
   * Given the backend's weeks of cash flow, already summed from its cash journal
   * When the chart draws them
   * Then each week is drawn with the backend's figures unchanged, in its order, labelled by the
   *      Monday it starts on in the player's language: the chart sums nothing of its own (#1204).
   */
  it("draws the backend's weeks as they are, labelled by their Monday", () => {
    render(<FinanceCashFlowChart weeks={weeks} incomeLabel="Income" expensesLabel="Expenses" />);

    expect(handedToCharts.data).toEqual([
      { week: formatDateShort("2026-08-03", "de"), income: 1_250_000, expenses: 980_000 },
      { week: formatDateShort("2026-08-10", "de"), income: 310_000, expenses: 40_000 },
    ]);
  });

  /**
   * Given weeks with figures in the millions
   * When the chart draws its axis
   * Then the labels read through the game's compact money formatter (its currency and its rule),
   *      not a private one, and the axis sizes itself to the label: a fixed width clips the longer
   *      forms some languages give ("-12,5 Mio. €").
   */
  it("labels the axis with the shared compact formatter and lets it size to the label", () => {
    render(<FinanceCashFlowChart weeks={weeks} incomeLabel="Income" expensesLabel="Expenses" />);

    expect(handedToCharts.tickFormatter?.(1_250_000)).toBe(formatVal(1_250_000));
    expect(handedToCharts.tickFormatter?.(-980_000)).toBe(formatVal(-980_000));
    expect(handedToCharts.yAxisWidth).toBe("auto");
  });

  /**
   * Given a figure that compact would round (941,250)
   * When the tooltip names it
   * Then it reads precisely: 1.25M for 1,250,000 and the exact figure where compact would lose it,
   *      since the tooltip is read deliberately, one figure at a time.
   */
  it("shows the tooltip figure compact but precise", () => {
    render(<FinanceCashFlowChart weeks={weeks} incomeLabel="Income" expensesLabel="Expenses" />);

    expect(handedToCharts.tooltipFormatter?.(1_250_000, "income")).toEqual([
      formatPreciseCompact(1_250_000),
      "income",
    ]);
    expect(handedToCharts.tooltipFormatter?.(941_250, "income")).toEqual([
      formatPreciseCompact(941_250),
      "income",
    ]);
    expect(formatPreciseCompact(941_250)).not.toBe(formatVal(941_250));
  });
});
