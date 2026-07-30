import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createPlayer } from "../../test-utils/factories";
import FinancesPayrollTable from "./FinancesPayrollTable";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "finances.until") return `Until ${params?.year}`;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

const roster = [
  createPlayer({ id: "player-1", full_name: "John Smith", wage: 520_000 }),
  createPlayer({ id: "player-2", full_name: "Ana Ruiz", wage: 310_000 }),
];

describe("FinancesPayrollTable", () => {
  it("selects a player when a row is clicked", () => {
    const onSelectPlayer = vi.fn();
    render(
      <FinancesPayrollTable roster={roster} onSelectPlayer={onSelectPlayer} />,
    );

    fireEvent.click(screen.getByRole("button", { name: /John Smith/ }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
  });

  it("selects a player from the keyboard with Enter and Space", () => {
    const onSelectPlayer = vi.fn();
    render(
      <FinancesPayrollTable roster={roster} onSelectPlayer={onSelectPlayer} />,
    );

    const row = screen.getByRole("button", { name: /Ana Ruiz/ });
    expect(row).toHaveAttribute("tabindex", "0");

    fireEvent.keyDown(row, { key: "Enter" });
    fireEvent.keyDown(row, { key: " " });

    expect(onSelectPlayer).toHaveBeenCalledTimes(2);
    expect(onSelectPlayer).toHaveBeenNthCalledWith(1, "player-2");
    expect(onSelectPlayer).toHaveBeenNthCalledWith(2, "player-2");
  });

  it("leaves rows out of the tab order when selection is unavailable", () => {
    render(<FinancesPayrollTable roster={roster} />);

    expect(screen.queryByRole("button", { name: /John Smith/ })).toBeNull();
  });
});
