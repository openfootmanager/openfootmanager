import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import PlayerProfileActionsMenu from "./PlayerProfileActionsMenu";
import { getSquad, setPlayerSquadRole, setStartingXi } from "../../services/squadService";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";

// Whether the backend's position table has arrived; a test says it has not.
const positionTableState = vi.hoisted(() => ({ arrived: true }));

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  const { createPositionTable } = await import("../../lib/positionTable");
  const canned = cannedPositionsModule();
  const notArrived = createPositionTable([]);
  return {
    ...canned,
    usePositionTable: () => {
      const arrived = canned.usePositionTable();
      return positionTableState.arrived ? arrived : notArrived;
    },
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../services/transfersService", () => ({
  toggleTransferList: vi.fn(),
  toggleLoanList: vi.fn(),
}));

vi.mock("../../services/squadService", () => ({
  getSquad: vi.fn(),
  setPlayerSquadRole: vi.fn(),
  setStartingXi: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

function buildPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "p1",
    full_name: "Test Player",
    team_id: "team-1",
    position: "Striker",
    date_of_birth: "1995-03-01",
    contract_end: "2028-06-30",
    wage: 5000,
    market_value: 1_000_000,
    transfer_listed: false,
    loan_listed: false,
    retired: false,
    squad_role: "Senior",
    injury: null,
    stats: {},
    career: [],
    ...overrides,
  } as unknown as PlayerData;
}

// The game the test built last: get_squad answers with its players, each with the verdict the
// backend would give a healthy senior.
let currentGame: GameStateData | null = null;

function buildGameState(players: PlayerData[]): GameStateData {
  currentGame = {
    manager: { team_id: "team-1" },
    teams: [
      {
        id: "team-1",
        name: "My Club",
        formation: "4-4-2",
        starting_xi_ids: [],
      },
      {
        id: "team-2",
        name: "Rival Club",
        formation: "4-4-2",
        starting_xi_ids: [],
      },
    ],
    players,
    staff: [],
    clock: { current_date: "2026-07-01" },
  } as unknown as GameStateData;
  return currentGame;
}

function baseProps(player: PlayerData, gameState: GameStateData) {
  return {
    player,
    gameState,
    isFreeAgent: false,
    hasLetExpireIntent: false,
    contractRiskLevel: "stable" as const,
    actionSubmitting: false,
    onGameUpdate: vi.fn(),
    onOpenRenewal: vi.fn(),
    onMarkLetExpire: vi.fn(),
    onClearLetExpire: vi.fn(),
    onOpenTermination: vi.fn(),
    onOpenBid: vi.fn(),
    onOpenFreeAgentContract: vi.fn(),
    onError: vi.fn(),
  };
}

function openMenu(): void {
  fireEvent.click(screen.getByText("common.actions"));
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getSquad).mockImplementation(async (teamId: string) =>
    (currentGame?.players ?? [])
      .filter((player) => player.team_id === teamId)
      .map((player) => ({ ...player, match_day_eligible: true })),
  );
});

describe("PlayerProfileActionsMenu", () => {
  it("shows contract, market and squad actions for the manager's own player", () => {
    const player = buildPlayer();
    const props = baseProps(player, buildGameState([player]));

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
    openMenu();

    expect(screen.getByText("common.renewContract")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.letContractExpire")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.terminateContract")).toBeInTheDocument();
    expect(screen.getByText("squad.addToTransferList")).toBeInTheDocument();
    expect(screen.getByText("squad.addToLoanList")).toBeInTheDocument();
    // sole available player is auto-slotted into the XI
    expect(screen.getByText("squad.sendToBench")).toBeInTheDocument();
  });

  /**
   * Given the manager's own player, in the XI, and the position table loaded or not
   * When the menu is opened
   * Then "Send to bench" is enabled once the table has loaded and disabled before: a lineup change
   *      is not made without it, and a disabled item says so where a silent no-op would not.
   */
  it("disables the lineup actions until the position table has loaded", async () => {
    const starters = Array.from({ length: 11 }, (_, i) =>
      buildPlayer({ id: `s${i}`, full_name: `Starter ${i}` }),
    );
    const bench = buildPlayer({ id: "bench", full_name: "Bench Player" });
    const game = buildGameState([...starters, bench]);
    (game.teams[0] as { starting_xi_ids: string[] }).starting_xi_ids = starters.map((p) => p.id);
    const props = baseProps(starters[0], game);

    positionTableState.arrived = false;
    try {
      const { unmount } = render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
      openMenu();
      expect(screen.getByRole("menuitem", { name: "squad.sendToBench" })).toBeDisabled();
      unmount();
    } finally {
      positionTableState.arrived = true;
    }

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
    await act(async () => {}); // the squad has answered for this revision
    openMenu();
    expect(screen.getByRole("menuitem", { name: "squad.sendToBench" })).toBeEnabled();
  });

  /**
   * Given a bench player's profile, and the position table not loaded
   * When the menu is opened
   * Then "Make starter" is disabled (loaded, it is enabled).
   */
  it("disables Make starter on a bench player's profile until the table has loaded", async () => {
    const starters = Array.from({ length: 11 }, (_, i) =>
      buildPlayer({ id: `s${i}`, full_name: `Starter ${i}` }),
    );
    const bench = buildPlayer({ id: "bench", full_name: "Bench Player" });
    const game = buildGameState([...starters, bench]);
    (game.teams[0] as { starting_xi_ids: string[] }).starting_xi_ids = starters.map((p) => p.id);
    const props = baseProps(bench, game);

    positionTableState.arrived = false;
    try {
      const { unmount } = render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
      openMenu();
      expect(screen.getByRole("menuitem", { name: "squad.makeStarter" })).toBeDisabled();
      unmount();
    } finally {
      positionTableState.arrived = true;
    }

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
    await act(async () => {}); // the squad has answered for this revision
    openMenu();
    expect(screen.getByRole("menuitem", { name: "squad.makeStarter" })).toBeEnabled();
  });

  /**
   * Given a bench player's profile, and get_squad has not answered for this revision
   * When the menu is opened and "Make starter" is clicked, and then the squad answers
   * Then it is disabled and saves nothing while the squad is pending, and enabled once it has
   *      answered: the eligibility the lineup is worked out from is the backend's, not a guess.
   */
  it("saves nothing from a profile promote while get_squad is in flight", async () => {
    const starters = Array.from({ length: 11 }, (_, i) =>
      buildPlayer({ id: `s${i}`, full_name: `Starter ${i}` }),
    );
    const bench = buildPlayer({ id: "bench", full_name: "Bench Player" });
    const game = buildGameState([...starters, bench]);
    (game.teams[0] as { starting_xi_ids: string[] }).starting_xi_ids = starters.map((p) => p.id);
    const squad = game.players.map((player) => ({ ...player, match_day_eligible: true }));
    let answer: (players: PlayerData[]) => void = () => {};
    vi.mocked(getSquad).mockReturnValue(new Promise((resolve) => (answer = resolve)));

    render(<PlayerProfileActionsMenu {...baseProps(bench, game)} isManagerOwnedProfile />);
    openMenu();
    const makeStarter = screen.getByRole("menuitem", { name: "squad.makeStarter" });
    expect(makeStarter).toBeDisabled();
    fireEvent.click(makeStarter);
    await act(async () => {});
    expect(setStartingXi).not.toHaveBeenCalled();

    await act(async () => answer(squad));
    expect(screen.getByRole("menuitem", { name: "squad.makeStarter" })).toBeEnabled();
  });

  it("toggles the transfer list and propagates the updated game state", async () => {
    const player = buildPlayer();
    const props = baseProps(player, buildGameState([player]));
    const updated = { updated: true } as unknown as GameStateData;
    vi.mocked(toggleTransferList).mockResolvedValue(updated);

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
    openMenu();
    fireEvent.click(screen.getByText("squad.addToTransferList"));

    await waitFor(() => {
      expect(toggleTransferList).toHaveBeenCalledWith("p1");
      expect(props.onGameUpdate).toHaveBeenCalledWith(updated);
    });
    expect(toggleLoanList).not.toHaveBeenCalled();
  });

  it("reports mutation failures through onError", async () => {
    const player = buildPlayer();
    const props = baseProps(player, buildGameState([player]));
    vi.mocked(toggleTransferList).mockRejectedValue(new Error("boom"));

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
    openMenu();
    fireEvent.click(screen.getByText("squad.addToTransferList"));

    await waitFor(() => {
      expect(props.onError).toHaveBeenCalled();
    });
    expect(props.onGameUpdate).not.toHaveBeenCalled();
  });

  it("delegates eligible under-21 players to the youth academy", async () => {
    const player = buildPlayer({ date_of_birth: "2007-01-01" });
    const props = baseProps(player, buildGameState([player]));
    const updated = { updated: true } as unknown as GameStateData;
    vi.mocked(setPlayerSquadRole).mockResolvedValue(updated);

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
    openMenu();
    fireEvent.click(screen.getByText("youthAcademy.delegateToYouthAcademy"));

    await waitFor(() => {
      expect(setPlayerSquadRole).toHaveBeenCalledWith("p1", "Youth");
      expect(props.onGameUpdate).toHaveBeenCalledWith(updated);
    });
  });

  it("offers a transfer bid for another club's player", () => {
    const player = buildPlayer({ id: "p2", team_id: "team-2" });
    const props = baseProps(player, buildGameState([player]));

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile={false} />);
    openMenu();

    fireEvent.click(screen.getByText("transfers.makeBid"));
    expect(props.onOpenBid).toHaveBeenCalled();
    expect(screen.queryByText("common.renewContract")).not.toBeInTheDocument();
  });

  it("offers a contract to free agents", () => {
    const player = buildPlayer({ id: "p3", team_id: null });
    const props = baseProps(player, buildGameState([player]));

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile={false} isFreeAgent />);
    openMenu();

    fireEvent.click(screen.getByText("transfers.offerContract"));
    expect(props.onOpenFreeAgentContract).toHaveBeenCalled();
  });

  /**
   * Given the manager's own player's profile
   * When the actions menu is opened and closed with Escape
   * Then the Actions button says so through aria-expanded, open and then closed again.
   */
  it("says on its button whether the actions menu is open", async () => {
    const player = buildPlayer();
    const props = baseProps(player, buildGameState([player]));

    render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);
    const button = screen.getByRole("button", { name: "common.playerActions" });
    expect(button).toHaveAttribute("aria-expanded", "false");

    openMenu();
    expect(await screen.findByRole("menu")).toBeInTheDocument();
    expect(button).toHaveAttribute("aria-expanded", "true");

    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(button).toHaveAttribute("aria-expanded", "false");
  });

  it("renders nothing for retired players outside the manager's club", () => {
    const player = buildPlayer({ id: "p4", team_id: null, retired: true });
    const props = baseProps(player, buildGameState([player]));

    const { container } = render(
      <PlayerProfileActionsMenu {...props} isManagerOwnedProfile={false} />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("renders nothing for retired players even when manager-owned", () => {
    const player = buildPlayer({ retired: true });
    const props = baseProps(player, buildGameState([player]));

    const { container } = render(<PlayerProfileActionsMenu {...props} isManagerOwnedProfile />);

    expect(container).toBeEmptyDOMElement();
  });
});
