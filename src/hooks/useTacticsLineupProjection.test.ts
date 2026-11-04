import { renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerData, TeamData } from "../store/gameStore";
import type { TacticsLineup } from "../store/generated/TacticsLineup";
import {
  resetTacticsLineupCache,
  tacticsLineupRevision,
  useTacticsLineupProjection,
} from "./useTacticsLineupProjection";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

const lineup: TacticsLineup = {
  formation: "4-4-2",
  eleven: [
    { position: "Goalkeeper", player_id: "gk", role: "SweeperKeeper", familiarity: "natural" },
  ],
  substitutes: ["sub"],
  reserves: ["hurt"],
};

const team = {
  id: "club",
  formation: "4-4-2",
  starting_xi_ids: ["gk"],
  player_roles: {},
} as unknown as TeamData;

const player = (id: string, overrides: Partial<PlayerData> = {}) =>
  ({ id, team_id: "club", injury: null, condition: 90, ...overrides }) as unknown as PlayerData;

describe("useTacticsLineupProjection", () => {
  beforeEach(() => {
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
  });

  /**
   * Given a club's revision
   * When the hook asks for its lineup and the backend answers
   * Then it is loaded with the backend's side, asked once.
   */
  it("loads the backend's side for the club, asking once", async () => {
    mockedInvoke.mockResolvedValue(lineup);

    const { result } = renderHook(() => useTacticsLineupProjection("rev-1"));

    expect(result.current.loaded).toBe(false);
    await waitFor(() => expect(result.current.loaded).toBe(true));
    expect(result.current.lineup).toEqual(lineup);
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(mockedInvoke).toHaveBeenCalledWith("get_tactics_lineup");
  });

  /**
   * Given a lineup loaded for one revision
   * When the club changes (a new revision) and its answer has not come yet
   * Then it is not loaded: an answer for the club as it was is not the club as it stands.
   */
  it("is not loaded for a new revision until its own answer arrives", async () => {
    mockedInvoke.mockResolvedValueOnce(lineup).mockReturnValueOnce(new Promise(() => {}));
    const { result, rerender } = renderHook(
      ({ revision }) => useTacticsLineupProjection(revision),
      {
        initialProps: { revision: "rev-1" },
      },
    );
    await waitFor(() => expect(result.current.loaded).toBe(true));

    rerender({ revision: "rev-2" });

    expect(result.current.loaded).toBe(false);
    expect(result.current.lineup).toBeNull();
  });

  /**
   * Given no club (an empty revision)
   * When the hook runs
   * Then nothing is asked and nothing is loaded.
   */
  it("asks nothing without a club", () => {
    const { result } = renderHook(() => useTacticsLineupProjection(""));

    expect(mockedInvoke).not.toHaveBeenCalled();
    expect(result.current.loaded).toBe(false);
  });
});

describe("tacticsLineupRevision", () => {
  /**
   * Given a club and its squad
   * When the saved eleven, a player's injury, the day or the formation changes
   * Then the revision changes, so the side is asked for again; with no club it is empty.
   */
  it("changes with everything the side depends on", () => {
    const squad = [player("gk"), player("sub")];
    const base = tacticsLineupRevision(1, team, squad, "2031-08-01");

    const changed = [
      tacticsLineupRevision(1, { ...team, starting_xi_ids: ["sub"] }, squad, "2031-08-01"),
      tacticsLineupRevision(
        1,
        team,
        [player("gk", { injury: { name: "Knock", days_remaining: 3 } }), player("sub")],
        "2031-08-01",
      ),
      tacticsLineupRevision(1, team, squad, "2031-08-02"),
      tacticsLineupRevision(1, { ...team, formation: "4-3-3" }, squad, "2031-08-01"),
      tacticsLineupRevision(2, team, squad, "2031-08-01"),
    ];

    for (const revision of changed) expect(revision).not.toBe(base);
    expect(tacticsLineupRevision(1, team, squad, "2031-08-01")).toBe(base);
    expect(tacticsLineupRevision(1, null, squad, "2031-08-01")).toBe("");
  });
});
