import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

import { useDigestAdvance } from "./useDigestAdvance";
import type { OneDayResponse } from "../services/advanceTimeService";

vi.mock("../services/advanceTimeService", () => ({
  advanceOneDay: vi.fn(),
}));
vi.mock("../components/dashboard/advanceRecap", () => ({
  buildAdvanceRecap: vi.fn().mockReturnValue({
    advancedTo: "2026-09-02",
    matches: [],
    transfers: [],
    news: [],
    inbox: [],
    hasEvents: false,
  }),
}));

const { advanceOneDay } = await import("../services/advanceTimeService");
const mockedAdvanceOneDay = vi.mocked(advanceOneDay);

function makeAdvancedResponse(date: string): OneDayResponse {
  return {
    action: "advanced",
    date,
    results: [],
    game: { clock: { current_date: `${date}T00:00:00Z` } } as never,
  };
}

describe("useDigestAdvance", () => {
  const setGameState = vi.fn();
  const onFired = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("starts not running and with empty entries", () => {
    const { result } = renderHook(() =>
      useDigestAdvance(setGameState, onFired),
    );
    expect(result.current.isRunning).toBe(false);
    expect(result.current.entries).toEqual([]);
    expect(result.current.stopReason).toBeNull();
    expect(result.current.isVisible).toBe(false);
  });

  it("accumulates entries for each advanced day until match_day stop", async () => {
    mockedAdvanceOneDay
      .mockResolvedValueOnce(makeAdvancedResponse("2026-09-01"))
      .mockResolvedValueOnce(makeAdvancedResponse("2026-09-02"))
      .mockResolvedValueOnce({ action: "match_day", date: "2026-09-03", results: [] });

    const { result } = renderHook(() =>
      useDigestAdvance(setGameState, onFired),
    );

    await act(async () => {
      await result.current.startDigest();
    });

    expect(result.current.isRunning).toBe(false);
    expect(result.current.entries).toHaveLength(2);
    expect(result.current.entries[0].date).toBe("2026-09-01");
    expect(result.current.entries[1].date).toBe("2026-09-02");
    expect(result.current.stopReason).toEqual({ kind: "match_day" });
    expect(result.current.isVisible).toBe(true);
  });

  it("stops with blocked reason and surfaces blockers", async () => {
    mockedAdvanceOneDay.mockResolvedValueOnce({
      action: "blocked",
      date: "2026-09-01",
      blockers: [{ id: "injured_xi", severity: "warn", text: "2 injured", tab: "Squad" }],
      results: [],
    });

    const { result } = renderHook(() =>
      useDigestAdvance(setGameState, onFired),
    );

    await act(async () => {
      await result.current.startDigest();
    });

    expect(result.current.stopReason).toMatchObject({ kind: "blocked" });
    if (result.current.stopReason?.kind === "blocked") {
      expect(result.current.stopReason.blockers).toHaveLength(1);
    }
    expect(result.current.entries).toHaveLength(0);
  });

  it("calls onFired and sets fired stop reason when manager is dismissed", async () => {
    mockedAdvanceOneDay.mockResolvedValueOnce({
      action: "fired",
      date: "2026-09-01",
      results: [],
      game: { clock: { current_date: "2026-09-02T00:00:00Z" } } as never,
    });

    const { result } = renderHook(() =>
      useDigestAdvance(setGameState, onFired),
    );

    await act(async () => {
      await result.current.startDigest();
    });

    expect(onFired).toHaveBeenCalledOnce();
    expect(result.current.stopReason).toEqual({ kind: "fired" });
  });

  it("dismissDigest resets all state", async () => {
    mockedAdvanceOneDay.mockResolvedValueOnce({ action: "match_day", date: "2026-09-01", results: [] });

    const { result } = renderHook(() =>
      useDigestAdvance(setGameState, onFired),
    );

    await act(async () => {
      await result.current.startDigest();
    });

    expect(result.current.stopReason).not.toBeNull();

    act(() => {
      result.current.dismissDigest();
    });

    expect(result.current.entries).toEqual([]);
    expect(result.current.stopReason).toBeNull();
    expect(result.current.isVisible).toBe(false);
  });
});
