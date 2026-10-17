import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useContractRiskLevels } from "./useContractWarningStages";

vi.mock("../services/contractService", () => ({ getContractWarningStages: vi.fn() }));

const { getContractWarningStages } = await import("../services/contractService");
const mockedStages = vi.mocked(getContractWarningStages);

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

    const { result } = renderHook(() => useContractRiskLevels(["near", "far"]));
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
    const { rerender } = renderHook(({ revision }) => useContractRiskLevels(["a"], revision), {
      initialProps: { revision: "2026-08-01" },
    });
    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(1));

    rerender({ revision: "2026-08-01" });
    expect(mockedStages).toHaveBeenCalledTimes(1);
    rerender({ revision: "2026-08-02" });
    await waitFor(() => expect(mockedStages).toHaveBeenCalledTimes(2));

    mockedStages.mockClear();
    renderHook(() => useContractRiskLevels([]));
    expect(mockedStages).not.toHaveBeenCalled();
  });
});
