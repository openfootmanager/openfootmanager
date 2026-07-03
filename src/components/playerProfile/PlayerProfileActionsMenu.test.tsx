import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import PlayerProfileActionsMenu from "./PlayerProfileActionsMenu";
import { setPlayerSquadRole } from "../../services/squadService";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

vi.mock("../../services/transfersService", () => ({
    toggleTransferList: vi.fn(),
    toggleLoanList: vi.fn(),
}));

vi.mock("../../services/squadService", () => ({
    setPlayerSquadRole: vi.fn(),
    setStartingXi: vi.fn(),
}));

vi.mock("react-i18next", () => ({
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

function buildGameState(players: PlayerData[]): GameStateData {
    return {
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

        render(
            <PlayerProfileActionsMenu
                {...props}
                isManagerOwnedProfile={false}
                isFreeAgent
            />,
        );
        openMenu();

        fireEvent.click(screen.getByText("transfers.offerContract"));
        expect(props.onOpenFreeAgentContract).toHaveBeenCalled();
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

        const { container } = render(
            <PlayerProfileActionsMenu {...props} isManagerOwnedProfile />,
        );

        expect(container).toBeEmptyDOMElement();
    });
});
