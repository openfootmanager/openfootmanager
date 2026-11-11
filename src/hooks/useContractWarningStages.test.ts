import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useContractRiskLevels } from "./useContractWarningStages";

vi.mock("../services/contractService", () => ({ getContractWarningStages: vi.fn() }));

const { getContractWarningStages } = await import("../services/contractService");
const mockedStages = vi.mocked(getContractWarningStages);

const player = (id: string, contractEnd: string | null = "2027-06-30") => ({
  id,
  contract_end: contractEnd,
});

beforeEach(() => {
  mockedStages.mockReset();
});

describe("useContractRiskLevels", () => {
  /**
   * Given the backend names a stage for one player and none for another
   * When the hook mounts with both
   * Then it reads "stable" until the answer, then the player's badge level from the backend's
   *      stage, whatever his contract date is.
   */
  it("reads each player's badge level from the backend's stage", async () => {
    mockedStages.mockResolvedValue({ near: "final_weeks", far: null });

    const { result } = renderHook(() => useContractRiskLevels([player("near"), player("far")]));
    expect(result.current("near")).toBe("stable");

    await waitFor(() => expect(result.current("near")).toBe("critical"));
    expect(result.current("far")).toBe("stable");
    expect(mockedStages).toHaveBeenCalledWith(["near", "far"]);
  });

  /**
   * Given the same players and a newer revision (a day passed)
   * When the hook renders again
   * Then the backend is asked again; the same revision asks once, and no players asks nothing.
   */
  it("asks again when the revision changes, and not for nobody", async () => {
    mockedStages.mockResolvedValue({ a: "twelve_months" });
    const { rerender } = renderHook(
      ({ revision }) => useContractRiskLevels([player("a")], revision),
      {
        initialProps: { revision: "2026-08-01" },
      },
    );
    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(1));

    rerender({ revision: "2026-08-01" });
    expect(mockedStages).toHaveBeenCalledTimes(1);
    rerender({ revision: "2026-08-02" });
    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(2));

    mockedStages.mockClear();
    renderHook(() => useContractRiskLevels([]));
    expect(mockedStages).not.toHaveBeenCalled();
  });

  /**
   * Given a player whose contract was renewed today (the date is the same, contract_end moved)
   * When the hook renders again
   * Then the backend is asked again, so the badge does not stay on the old stage until tomorrow.
   */
  it("asks again when a contract changes on the same day", async () => {
    mockedStages.mockResolvedValue({ a: "final_weeks" });
    const { rerender } = renderHook(
      ({ contractEnd }) => useContractRiskLevels([player("a", contractEnd)], "2026-08-01"),
      { initialProps: { contractEnd: "2026-08-20" } },
    );
    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(1));

    mockedStages.mockResolvedValue({ a: null });
    rerender({ contractEnd: "2028-06-30" });

    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(2));
  });

  /**
   * Given a player marked to let his contract expire today (same date, same contract_end)
   * When the hook renders again
   * Then the backend is asked again.
   */
  it("asks again when an expiry mark is set on the same day", async () => {
    mockedStages.mockResolvedValue({ a: "final_weeks" });
    const unmarked = player("a");
    const marked = {
      ...unmarked,
      morale_core: {
        manager_trust: 50,
        renewal_state: {
          status: "Idle" as const,
          conversation_round: 0,
          exit_intent: { kind: "let_expire" as const, set_on: "2026-08-01" },
        },
      },
    };
    const { rerender } = renderHook(
      ({ current }) => useContractRiskLevels([current], "2026-08-01"),
      { initialProps: { current: unmarked as typeof marked } },
    );
    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(1));

    rerender({ current: marked });

    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(2));
  });
});
