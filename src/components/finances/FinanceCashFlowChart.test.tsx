import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import { formatVal } from "../../lib/helpers";
import type { FinancialTransactionData } from "../../store/types";
import { FinanceCashFlowChart } from "./FinanceCashFlowChart";

// What the chart hands recharts for its axis and tooltip, captured: jsdom has no layout, so the
// real charts draw nothing to read the labels from.
const handedToCharts: {
  tickFormatter?: (value: number) => string;
  tooltipFormatter?: (value: unknown, name: unknown) => [string, string];
} = {};

vi.mock("recharts", () => {
  const Passthrough = ({ children }: { children?: ReactNode }) => <div>{children}</div>;
  return {
    ResponsiveContainer: Passthrough,
    AreaChart: Passthrough,
    Area: () => null,
    XAxis: () => null,
    CartesianGrid: () => null,
    Legend: () => null,
    YAxis: (props: { tickFormatter: (value: number) => string }) => {
      handedToCharts.tickFormatter = props.tickFormatter;
      return null;
    },
    Tooltip: (props: { formatter: (value: unknown, name: unknown) => [string, string] }) => {
      handedToCharts.tooltipFormatter = props.formatter;
      return null;
    },
  };
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const ledger: FinancialTransactionData[] = [
  { date: "2026-08-03", description: "gate", amount: 1_250_000, kind: "PrizeMoney" },
  { date: "2026-08-04", description: "wages", amount: -980_000, kind: "ContractTermination" },
];

describe("FinanceCashFlowChart", () => {
  /**
   * Given a ledger with figures in the millions
   * When the chart draws its axis and its tooltip
   * Then both read through the game's one money formatter (its currency and its compact rule),
   *      not a private one that drops the currency and rounds on its own.
   */
  it("labels the axis and the tooltip with the shared money formatter", () => {
    render(<FinanceCashFlowChart ledger={ledger} incomeLabel="Income" expensesLabel="Expenses" />);

    expect(handedToCharts.tickFormatter?.(1_250_000)).toBe(formatVal(1_250_000));
    expect(handedToCharts.tickFormatter?.(-980_000)).toBe(formatVal(-980_000));
    expect(handedToCharts.tooltipFormatter?.(1_250_000, "income")).toEqual([
      formatVal(1_250_000),
      "income",
    ]);
  });
});
