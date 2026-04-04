import { describe, expect, it } from "vitest";
import { planAutoSelectSwaps } from "./preMatchSetupUtils";
import type { EnginePlayerData } from "./types";

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