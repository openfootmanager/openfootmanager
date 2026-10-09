import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getScoutedReport } from "../../services/scoutingService";
import type { ScoutedPlayerView } from "../../store/gameStore";
import { useScoutedReport } from "./useScoutedReport";

vi.mock("../../services/scoutingService", () => ({
  getScoutedReport: vi.fn(),
}));

function reportFor(playerId: string): ScoutedPlayerView {
  return {
    player_id: playerId,
    scouted_on: "2026-09-01",
    attributes: {} as ScoutedPlayerView["attributes"],
    out_of_date: false,
  };
}

describe("useScoutedReport", () => {
  beforeEach(() => {
    vi.mocked(getScoutedReport).mockReset();
  });

  /**
   * Given a profile showing a scouted player A
   * When the same mounted profile is handed player B, whose report is still loading
   * Then not even the first render for B shows A's report
   */
  it("never shows the previous player's report for the next player", async () => {
    vi.mocked(getScoutedReport).mockImplementation((playerId) =>
      playerId === "a" ? Promise.resolve(reportFor("a")) : new Promise(() => {}),
    );
    const rendered: Array<ScoutedPlayerView | null> = [];
    const { rerender } = renderHook(
      ({ playerId }) => {
        const report = useScoutedReport(playerId, true, "2026-10-01");
        rendered.push(report);
        return report;
      },
      { initialProps: { playerId: "a" } },
    );
    await waitFor(() => expect(rendered[rendered.length - 1]?.player_id).toBe("a"));
    const rendersBeforeSwitch = rendered.length;

    rerender({ playerId: "b" });

    expect(rendered.slice(rendersBeforeSwitch).every((report) => report === null)).toBe(true);
  });

  /**
   * Given a player who was not scouted
   * When the report is requested
   * Then there is none
   */
  it("returns null when the backend has no report", async () => {
    vi.mocked(getScoutedReport).mockResolvedValue(null);

    const { result } = renderHook(() => useScoutedReport("a", true, "2026-10-01"));

    await waitFor(() => expect(getScoutedReport).toHaveBeenCalledWith("a"));
    expect(result.current).toBeNull();
  });
});
