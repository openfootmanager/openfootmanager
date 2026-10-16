import { renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useClubLeagueStanding } from "./useClubLeagueStanding";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("useClubLeagueStanding", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /**
   * Given no club (a clubless career, or no game loaded yet)
   * When the hook runs
   * Then the backend is not asked for a table.
   */
  it("does not ask the backend about a club that does not exist", () => {
    renderHook(() => useClubLeagueStanding("", "2026-08-10"));

    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  /**
   * Given a club
   * When the hook runs
   * Then the backend's standing for that club is returned.
   */
  it("returns the backend's standing for the club", async () => {
    const standing = { competition_id: "league-1", competition_name: "League", league_pos: 3 };
    mockedInvoke.mockResolvedValue(standing);

    const { result } = renderHook(() => useClubLeagueStanding("team-1", "2026-08-10"));

    await waitFor(() => expect(result.current.standing).toEqual(standing));
    expect(mockedInvoke).toHaveBeenCalledWith("get_club_league_standing", { teamId: "team-1" });
  });
});
