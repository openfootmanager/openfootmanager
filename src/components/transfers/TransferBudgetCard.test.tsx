import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { formatExactMoney } from "../../lib/helpers";
import { TransferBudgetCard } from "./TransferBudgetCard";

vi.mock("../../services/financeService", () => ({
  getFinanceSnapshot: vi.fn(async () => ({
    snapshot: {
      committedTransferFees: 800000,
      availableTransferBudget: 200000,
    },
  })),
}));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));

describe("transfer budget commitments", () => {
  /** Given a backend budget reduced by an agreed fee, when the card loads, then both committed and available money are visible. */
  it("shows committed fees and the available budget returned by Rust", async () => {
    render(<TransferBudgetCard teamId="buyer" revision={{}} />);
    await waitFor(() => expect(screen.getByText(formatExactMoney(200000))).toBeInTheDocument());
    expect(screen.getByText("finances.availableTransferBudget")).toBeInTheDocument();
    expect(screen.getByText(formatExactMoney(800000))).toBeInTheDocument();
    expect(screen.queryByText(/€800K|€200K/)).not.toBeInTheDocument();
  });
});
