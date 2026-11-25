import { render, screen, within } from "@testing-library/react";
import { it, expect, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { FinanceOverview } from "./FinanceOverview";
import { createTeam } from "../../test-utils/factories";
import { getFinanceSnapshot } from "../../services/financeService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init() {} },
  useTranslation: () => ({ t: (key: string) => key }),
}));

/** Given a gross budget of 300k and 80k reserved, when the real service maps a snapshot, then Finances displays 220k available and 80k committed. */
it("displays the backend budget after commitments instead of the gross envelope", async () => {
  vi.mocked(invoke).mockResolvedValue({
    snapshot: {
      committed_transfer_fees: 80000,
      available_transfer_budget: 220000,
      cash_flow: [],
    },
  });
  const { snapshot } = await getFinanceSnapshot("buyer");
  render(
    <FinanceOverview
      myTeam={createTeam({ transfer_budget: 300000 })}
      snapshot={snapshot}
      totalValue={0}
    />,
  );
  const available = screen.getByText("finances.availableTransferBudget").parentElement;
  const committed = screen.getByText("finances.committedTransferFees").parentElement;
  if (!available || !committed) throw new Error("Budget labels must be rendered");
  expect(within(available).getByText(/220/)).toBeInTheDocument();
  expect(within(committed).getByText(/80/)).toBeInTheDocument();
});
