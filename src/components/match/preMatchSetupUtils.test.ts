import { describe, expect, it, vi } from "vitest";
import {
    applyPlannedPreMatchSwaps,
    applySetPieceAssignments,
    getAutoSelectedSetPieceAssignments,
    planAutoSelectSwaps,
} from "./preMatchSetupUtils";
import type { EnginePlayerData, MatchSnapshot } from "./types";

const makePlayer = (overrides: Partial<EnginePlayerData> = {}): EnginePlayerData => ({
    id: "p1",
    name: "Test Player",
    position: "Midfielder",
    condition: 100,
    pace: 70,
    stamina: 70,
    strength: 70,
    agility: 70,
    passing: 70,
    shooting: 70,
    tackling: 70,
    dribbling: 70,
    defending: 70,
    positioning: 70,
    vision: 70,
    decisions: 70,
    composure: 70,
    aggression: 50,
    teamwork: 70,
    leadership: 50,
    handling: 30,
    reflexes: 30,
    aerial: 50,
    traits: [],
    ...overrides,
});

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

describe("planAutoSelectSwaps", () => {
    it("prefers a fitter, stronger bench player when a formation role is underpowered", () => {
        const starters = [
            makePlayer({ id: "gk-1", position: "Goalkeeper", handling: 80, reflexes: 80 }),
            makePlayer({ id: "d-weak", position: "Defender", condition: 60, defending: 55, tackling: 55, positioning: 55, strength: 55 }),
            makePlayer({ id: "d-2", position: "Defender", defending: 76, tackling: 75, positioning: 74, strength: 73 }),
            makePlayer({ id: "d-3", position: "Defender", defending: 74, tackling: 74, positioning: 73, strength: 72 }),
            makePlayer({ id: "d-4", position: "Defender", defending: 72, tackling: 71, positioning: 70, strength: 69 }),
            makePlayer({ id: "m-1", position: "Midfielder", passing: 78, vision: 76, decisions: 74 }),
            makePlayer({ id: "m-2", position: "Midfielder", passing: 76, vision: 74, decisions: 72 }),
            makePlayer({ id: "m-3", position: "Midfielder", passing: 74, vision: 72, decisions: 70 }),
            makePlayer({ id: "m-4", position: "Midfielder", passing: 72, vision: 70, decisions: 68 }),
            makePlayer({ id: "f-1", position: "Forward", shooting: 76, positioning: 74, pace: 73 }),
            makePlayer({ id: "f-2", position: "Forward", shooting: 74, positioning: 72, pace: 71 }),
        ];
        const bench = [
            makePlayer({ id: "d-strong", position: "Defender", condition: 95, defending: 80, tackling: 79, positioning: 78, strength: 77 }),
        ];

        const swaps = planAutoSelectSwaps(starters, bench, {
            Goalkeeper: 1,
            Defender: 4,
            Midfielder: 4,
            Forward: 2,
        });

        expect(swaps).toContainEqual({
            playerOffId: "d-weak",
            playerOnId: "d-strong",
        });
    });

    it("returns no swaps when the current starters already satisfy the best lineup", () => {
        const starters = [
            makePlayer({ id: "gk-1", position: "Goalkeeper", handling: 82, reflexes: 82 }),
            makePlayer({ id: "d-1", position: "Defender", defending: 80, tackling: 80, positioning: 78, strength: 79 }),
            makePlayer({ id: "m-1", position: "Midfielder", passing: 80, vision: 78, decisions: 77 }),
            makePlayer({ id: "f-1", position: "Forward", shooting: 82, positioning: 80, pace: 79 }),
        ];
        const bench = [
            makePlayer({ id: "bench-1", position: "Forward", shooting: 60, positioning: 60, pace: 60 }),
        ];

        const swaps = planAutoSelectSwaps(starters, bench, {
            Goalkeeper: 1,
            Defender: 1,
            Midfielder: 1,
            Forward: 1,
        });

        expect(swaps).toEqual([]);
    });
});

describe("getAutoSelectedSetPieceAssignments", () => {
    it("returns non-null assignments in the expected application order", () => {
        expect(
            getAutoSelectedSetPieceAssignments({
                captain: "captain-1",
                penalty_taker: "penalty-1",
                free_kick_taker: null,
                corner_taker: "corner-1",
            }),
        ).toEqual([
            { role: "captain", playerId: "captain-1" },
            { role: "penalty", playerId: "penalty-1" },
            { role: "corner", playerId: "corner-1" },
        ]);
    });
});

describe("applyPlannedPreMatchSwaps", () => {
    it("applies swaps in order and returns the last snapshot", async () => {
        const firstSnapshot = makeSnapshot({ current_minute: 1 });
        const finalSnapshot = makeSnapshot({ current_minute: 2 });
        const onSnapshot = vi.fn();
        const applySwap = vi
            .fn()
            .mockResolvedValueOnce(firstSnapshot)
            .mockResolvedValueOnce(finalSnapshot);

        const result = await applyPlannedPreMatchSwaps(
            [
                { playerOffId: "a", playerOnId: "b" },
                { playerOffId: "c", playerOnId: "d" },
            ],
            applySwap,
            onSnapshot,
        );

        expect(applySwap.mock.calls).toEqual([
            [{ playerOffId: "a", playerOnId: "b" }],
            [{ playerOffId: "c", playerOnId: "d" }],
        ]);
        expect(onSnapshot).toHaveBeenNthCalledWith(1, firstSnapshot);
        expect(onSnapshot).toHaveBeenNthCalledWith(2, finalSnapshot);
        expect(result).toBe(finalSnapshot);
    });
});

describe("applySetPieceAssignments", () => {
    it("applies assignments in order and reports each snapshot", async () => {
        const captainSnapshot = makeSnapshot({
            home_set_pieces: {
                captain: "captain-1",
                penalty_taker: null,
                free_kick_taker: null,
                corner_taker: null,
            },
        });
        const cornerSnapshot = makeSnapshot({
            home_set_pieces: {
                captain: "captain-1",
                penalty_taker: null,
                free_kick_taker: null,
                corner_taker: "corner-1",
            },
        });
        const onSnapshot = vi.fn();
        const applyAssignment = vi
            .fn()
            .mockResolvedValueOnce(captainSnapshot)
            .mockResolvedValueOnce(cornerSnapshot);

        const result = await applySetPieceAssignments(
            [
                { role: "captain", playerId: "captain-1" },
                { role: "corner", playerId: "corner-1" },
            ],
            applyAssignment,
            onSnapshot,
        );

        expect(applyAssignment.mock.calls).toEqual([
            [{ role: "captain", playerId: "captain-1" }],
            [{ role: "corner", playerId: "corner-1" }],
        ]);
        expect(onSnapshot).toHaveBeenNthCalledWith(1, captainSnapshot);
        expect(onSnapshot).toHaveBeenNthCalledWith(2, cornerSnapshot);
        expect(result).toBe(cornerSnapshot);
    });
});