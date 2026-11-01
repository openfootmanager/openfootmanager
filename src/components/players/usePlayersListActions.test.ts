import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerSummary } from "../../services/playersService";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";
import type { GameStateData } from "../../store/gameStore";
import { usePlayersListActions } from "./usePlayersListActions";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock("../transfers/useTransferBidFlow", () => ({
  useTransferBidFlow: () => ({ openBidNegotiation: vi.fn() }),
}));
vi.mock("../transfers/useFreeAgentContractFlow", () => ({
  useFreeAgentContractFlow: () => ({ openFreeAgentContract: vi.fn() }),
}));
vi.mock("../../services/transfersService", () => ({
  toggleTransferList: vi.fn(),
  toggleLoanList: vi.fn(),
}));
vi.mock("../../services/scoutingService", () => ({ sendScout: vi.fn() }));

const gameState = {
  manager: { team_id: "mine" },
  players: [],
  staff: [],
  scouting_assignments: [],
  youth_scouting_assignments: [],
} as unknown as GameStateData;

const summary = (over: Partial<PlayerSummary>): PlayerSummary =>
  ({
    id: "p1",
    team_id: "mine",
    transfer_listed: false,
    loan_listed: false,
    retired: false,
    ...over,
  }) as PlayerSummary;

function menuLabels(player: PlayerSummary): string[] {
  const { result } = renderHook(() =>
    usePlayersListActions({
      gameState,
      onSelectPlayer: vi.fn(),
      onSelectTeam: vi.fn(),
      refetchSlice: vi.fn(),
    }),
  );
  return result.current.contextItemsFor(player).map((item) => item.label);
}

describe("usePlayersListActions context menu", () => {
  /**
   * Given a player of the manager's own club
   * When his context menu is built
   * Then it offers listing him for transfer and for loan, and no bid or scouting.
   */
  it("offers the club's own player the listing actions, not a bid", () => {
    const labels = menuLabels(summary({ team_id: "mine" }));

    expect(labels).toContain("squad.addToTransferList");
    expect(labels).toContain("squad.addToLoanList");
    expect(labels).not.toContain("transfers.makeBid");
    expect(labels).not.toContain("scouting.noScoutsFree");
  });

  /**
   * Given another club's player
   * When his context menu is built
   * Then it offers a bid and scouting, and no listing actions.
   */
  it("offers another club's player a bid and scouting, not the listing actions", () => {
    const labels = menuLabels(summary({ team_id: "rivals" }));

    expect(labels).toContain("transfers.makeBid");
    // No scout is on the fixture's staff, so the scouting action reads "no scouts free".
    expect(labels).toContain("scouting.noScoutsFree");
    expect(labels).not.toContain("squad.addToTransferList");
  });

  /**
   * Given a free agent, and a retired free agent
   * When their context menus are built
   * Then the free agent can be offered a contract and scouted; the retired one neither.
   */
  it("offers a free agent a contract, and a retired one nothing", () => {
    const free = menuLabels(summary({ team_id: null }));
    expect(free).toContain("transfers.offerContract");
    expect(free).toContain("scouting.noScoutsFree");

    const retired = menuLabels(summary({ team_id: null, retired: true }));
    expect(retired).not.toContain("transfers.offerContract");
    expect(retired).not.toContain("scouting.noScoutsFree");
  });
});

describe("usePlayersListActions list toggles", () => {
  function setup() {
    const onGameUpdate = vi.fn();
    const refetchSlice = vi.fn();
    const hook = renderHook(() =>
      usePlayersListActions({
        gameState,
        onGameUpdate,
        onSelectPlayer: vi.fn(),
        onSelectTeam: vi.fn(),
        refetchSlice,
      }),
    );
    const press = async (label: string) => {
      const item = hook.result.current
        .contextItemsFor(summary({ team_id: "mine" }))
        .find((entry) => entry.label === label);
      await act(async () => {
        await item?.onClick?.();
      });
    };
    return { hook, press, onGameUpdate, refetchSlice };
  }

  /**
   * Given a save that fails
   * When the manager lists his player for transfer, and for loan, from the menu
   * Then the failure is shown to him (it is not swallowed), and nothing is reported as updated.
   */
  it.each([
    ["squad.addToTransferList", toggleTransferList],
    ["squad.addToLoanList", toggleLoanList],
  ] as const)("shows the failure of %s", async (label, toggle) => {
    vi.mocked(toggle).mockRejectedValueOnce(new Error("disk full"));
    const { hook, press, onGameUpdate } = setup();

    await press(label);

    expect(hook.result.current.actionError).toBe("disk full");
    expect(onGameUpdate).not.toHaveBeenCalled();
  });

  /**
   * Given an earlier failure that is on screen
   * When the next list action succeeds
   * Then the error is cleared and the list is asked to reload.
   */
  it("clears the failure once a later toggle succeeds", async () => {
    vi.mocked(toggleTransferList).mockRejectedValueOnce(new Error("disk full"));
    vi.mocked(toggleTransferList).mockResolvedValueOnce({} as GameStateData);
    const { hook, press, refetchSlice } = setup();

    await press("squad.addToTransferList");
    expect(hook.result.current.actionError).toBe("disk full");
    await press("squad.addToTransferList");

    expect(hook.result.current.actionError).toBeNull();
    expect(refetchSlice).toHaveBeenCalled();
  });
});
