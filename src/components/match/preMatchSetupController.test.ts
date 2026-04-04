import { beforeEach, describe, expect, it, vi } from "vitest";

import {
    applyPreMatchFormationChange,
    applyPreMatchSwap,
    autoAssignPreMatchSetPieces,
} from "./preMatchSetupController";
import type { MatchSnapshot } from "./types";

const autoSelectSetPiecesMock = vi.fn();
const changeMatchFormationMock = vi.fn();
const changeMatchPlayStyleMock = vi.fn();
const setMatchSetPieceTakerMock = vi.fn();
const swapPreMatchPlayersMock = vi.fn();

vi.mock("../../services/liveMatchService", () => ({
    autoSelectSetPieces: (...args: unknown[]) => autoSelectSetPiecesMock(...args),
    changeMatchFormation: (...args: unknown[]) => changeMatchFormationMock(...args),
    changeMatchPlayStyle: (...args: unknown[]) => changeMatchPlayStyleMock(...args),
    setMatchSetPieceTaker: (...args: unknown[]) => setMatchSetPieceTakerMock(...args),
    swapPreMatchPlayers: (...args: unknown[]) => swapPreMatchPlayersMock(...args),
}));

const makeSnapshot = (overrides: Partial<MatchSnapshot> = {}): MatchSnapshot => ({
    phase: "PreKickOff",
    current_minute: 0,
    home_score: 0,
    away_score: 0,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: {
        id: "home-1",
        name: "Home FC",
        formation: "4-4-2",
        play_style: "Balanced",
        players: [],
    },
    away_team: {
        id: "away-1",
        name: "Away FC",
        formation: "4-4-2",
        play_style: "Balanced",
        players: [],
    },
    home_bench: [],
    away_bench: [],
    home_possession_pct: 50,
    away_possession_pct: 50,
    events: [],
    home_subs_made: 0,
    away_subs_made: 0,
    max_subs: 5,
    home_set_pieces: {
        captain: null,
        penalty_taker: null,
        free_kick_taker: null,
        corner_taker: null,
    },
    away_set_pieces: {
        captain: null,
        penalty_taker: null,
        free_kick_taker: null,
        corner_taker: null,
    },
    substitutions: [],
    allows_extra_time: false,
    home_yellows: {},
    away_yellows: {},
    sent_off: [],
    ...overrides,
});

describe("preMatchSetupController", () => {
    beforeEach(() => {
        autoSelectSetPiecesMock.mockReset();
        changeMatchFormationMock.mockReset();
        changeMatchPlayStyleMock.mockReset();
        setMatchSetPieceTakerMock.mockReset();
        swapPreMatchPlayersMock.mockReset();
    });

    it("applies a formation change and reports the updated snapshot", async () => {
        const snapshot = makeSnapshot({ current_minute: 12 });
        const onUpdateSnapshot = vi.fn();
        changeMatchFormationMock.mockResolvedValue(snapshot);

        const result = await applyPreMatchFormationChange(
            "Home",
            "4-3-3",
            onUpdateSnapshot,
        );

        expect(changeMatchFormationMock).toHaveBeenCalledWith("Home", "4-3-3");
        expect(onUpdateSnapshot).toHaveBeenCalledWith(snapshot);
        expect(result).toBe(snapshot);
    });

    it("skips the swap command when no starter is selected", async () => {
        const onUpdateSnapshot = vi.fn();

        const result = await applyPreMatchSwap(
            "Home",
            null,
            "bench-1",
            onUpdateSnapshot,
        );

        expect(swapPreMatchPlayersMock).not.toHaveBeenCalled();
        expect(onUpdateSnapshot).not.toHaveBeenCalled();
        expect(result).toBeNull();
    });

    it("applies auto-selected set pieces in the planned order", async () => {
        const captainSnapshot = makeSnapshot({
            home_set_pieces: {
                captain: "gk-1",
                penalty_taker: null,
                free_kick_taker: null,
                corner_taker: null,
            },
        });
        const penaltySnapshot = makeSnapshot({
            home_set_pieces: {
                captain: "gk-1",
                penalty_taker: "f-1",
                free_kick_taker: null,
                corner_taker: null,
            },
        });
        const cornerSnapshot = makeSnapshot({
            home_set_pieces: {
                captain: "gk-1",
                penalty_taker: "f-1",
                free_kick_taker: null,
                corner_taker: "m-1",
            },
        });
        const onUpdateSnapshot = vi.fn();

        autoSelectSetPiecesMock.mockResolvedValue({
            captain: "gk-1",
            penalty_taker: "f-1",
            free_kick_taker: null,
            corner_taker: "m-1",
        });
        setMatchSetPieceTakerMock
            .mockResolvedValueOnce(captainSnapshot)
            .mockResolvedValueOnce(penaltySnapshot)
            .mockResolvedValueOnce(cornerSnapshot);

        const result = await autoAssignPreMatchSetPieces(
            "Home",
            ["gk-1", "d-1", "m-1", "f-1"],
            onUpdateSnapshot,
        );

        expect(autoSelectSetPiecesMock).toHaveBeenCalledWith([
            "gk-1",
            "d-1",
            "m-1",
            "f-1",
        ]);
        expect(setMatchSetPieceTakerMock.mock.calls).toEqual([
            ["Home", "captain", "gk-1"],
            ["Home", "penalty", "f-1"],
            ["Home", "corner", "m-1"],
        ]);
        expect(onUpdateSnapshot).toHaveBeenNthCalledWith(1, captainSnapshot);
        expect(onUpdateSnapshot).toHaveBeenNthCalledWith(2, penaltySnapshot);
        expect(onUpdateSnapshot).toHaveBeenNthCalledWith(3, cornerSnapshot);
        expect(result).toBe(cornerSnapshot);
    });
});