import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PositionGroupEntry } from "../services/positionService";
import { resetPositionGroupsCache, usePositionGroups } from "./usePositionGroups";

vi.mock("../services/positionService", () => ({ getPositionGroups: vi.fn() }));

const { getPositionGroups } = await import("../services/positionService");
const mockedGroups = vi.mocked(getPositionGroups);

const table = (): PositionGroupEntry[] => [
  { position: "RightWingBack", group: "Defender" },
  { position: "Striker", group: "Forward" },
];

beforeEach(() => {
  mockedGroups.mockReset();
  resetPositionGroupsCache();
});

describe("usePositionGroups", () => {
  /**
   * Given the backend's table of position groups
   * When the hook mounts and a position is read
   * Then it has no group until the table arrives, then the table's group.
   */
  it("reads a position's group from the backend's table", async () => {
    mockedGroups.mockResolvedValue(table());

    const { result } = renderHook(() => usePositionGroups());
    expect(result.current("RightWingBack")).toBeNull();

    await waitFor(() => expect(result.current("RightWingBack")).toBe("Defender"));
    expect(result.current("Striker")).toBe("Forward");
    expect(result.current("Unheard Of")).toBeNull();
  });

  /**
   * Given several mounts of the hook, as a table of badges has
   * When they all mount
   * Then the backend is asked once.
   */
  it("asks the backend once for however many mounts", async () => {
    mockedGroups.mockResolvedValue(table());

    const rows = [1, 2, 3].map(() => renderHook(() => usePositionGroups()));
    await waitFor(() => expect(rows[2].result.current("Striker")).toBe("Forward"));

    expect(mockedGroups).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a first request that fails
   * When a screen mounts the hook again
   * Then it asks again instead of keeping the failure, and the first mount gets the table too.
   */
  it("asks again after a failed request and hands the table to every mount", async () => {
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    mockedGroups.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(table());

    const first = renderHook(() => usePositionGroups());
    await waitFor(() => expect(quiet).toHaveBeenCalled());
    const second = renderHook(() => usePositionGroups());

    await waitFor(() => expect(second.result.current("Striker")).toBe("Forward"));
    await waitFor(() => expect(first.result.current("Striker")).toBe("Forward"));
    expect(mockedGroups).toHaveBeenCalledTimes(2);
    quiet.mockRestore();
  });

  /**
   * Given a backend answer that is not a table (a screen test's stand-in, or a broken reply)
   * When the hook reads a position
   * Then it names no group instead of throwing, so a badge still draws.
   */
  it("names no group when the answer is not a table", async () => {
    mockedGroups.mockResolvedValue({ unexpected: true } as unknown as PositionGroupEntry[]);

    const { result } = renderHook(() => usePositionGroups());
    await act(async () => {
      await mockedGroups.mock.results[0]?.value;
    });

    expect(result.current("Striker")).toBeNull();
  });
});
