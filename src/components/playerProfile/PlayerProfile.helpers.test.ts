import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { PlayerData, TeamData } from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import {
    buildPlayerAdvancedStats,
    formatPlayerMarketValue,
    formatPlayerWage,
    getAttributeColorClass,
    getPlayerAge,
    getPlayerTeamName,
    resolvePlayerInjuryName,
} from "./PlayerProfile.helpers";

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
    return {
        id: "team-1",
        name: "Alpha FC",
        short_name: "ALP",
        country: "GB",
        city: "London",
        stadium_name: "Alpha Ground",
        stadium_capacity: 30000,
        finance: 500000,
        manager_id: "manager-1",
        reputation: 50,
        wage_budget: 50000,
        transfer_budget: 250000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-4-2",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#000000", secondary: "#ffffff" },
        starting_xi_ids: [],
        form: [],
        history: [],
        ...overrides,
    };
}

const originalSettings = useSettingsStore.getState().settings;

beforeEach(() => {
    useSettingsStore.setState({
        settings: { ...originalSettings, currency: "EUR", language: "en" },
    });
});

afterEach(() => {
    useSettingsStore.setState({ settings: originalSettings });
});

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
    return {
        id: "player-1",
        match_name: "J. Smith",
        full_name: "John Smith",
        date_of_birth: "2000-01-01",
        nationality: "GB",
        position: "Forward",
        natural_position: "Forward",
        alternate_positions: [],
        training_focus: null,
        attributes: {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 20,
            reflexes: 20,
            aerial: 60,
        },
        condition: 80,
        morale: 75,
        injury: null,
        team_id: "team-1",
        contract_end: "2026-10-15",
        wage: 12000,
        market_value: 350000,
        stats: {
            appearances: 10,
            goals: 4,
            assists: 3,
            clean_sheets: 0,
            yellow_cards: 1,
            red_cards: 0,
            avg_rating: 7.2,
            minutes_played: 450,
            shots: 20,
            shots_on_target: 10,
            passes_completed: 80,
            passes_attempted: 100,
            tackles_won: 9,
            interceptions: 6,
            fouls_committed: 5,
        },
        career: [],
        transfer_listed: false,
        loan_listed: false,
        transfer_offers: [],
        traits: [],
        ...overrides,
    };
}

describe("PlayerProfile.helpers", function (): void {
    it("resolves the player team name with free-agent and unknown fallbacks", function (): void {
        const teams = [createTeam()];

        expect(
            getPlayerTeamName(teams, "team-1", {
                freeAgent: "Free Agent",
                unknown: "Unknown",
            }),
        ).toBe("Alpha FC");
        expect(
            getPlayerTeamName(teams, null, {
                freeAgent: "Free Agent",
                unknown: "Unknown",
            }),
        ).toBe("Free Agent");
        expect(
            getPlayerTeamName(teams, "team-2", {
                freeAgent: "Free Agent",
                unknown: "Unknown",
            }),
        ).toBe("Unknown");
    });

    it("calculates age relative to an as-of date instead of just the birth year", function (): void {
        expect(getPlayerAge("2000-07-02", "2026-07-01")).toBe(25);
        expect(getPlayerAge("2000-07-01", "2026-07-01")).toBe(26);
    });

    it("formats market values across value ranges", function (): void {
        expect(formatPlayerMarketValue(999)).toBe("€999");
        expect(formatPlayerMarketValue(125000)).toBe("€125K");
        expect(formatPlayerMarketValue(2500000)).toBe("€2.5M");
    });

    it("formats annual wages as weekly display values", function (): void {
        expect(formatPlayerWage(52000, "/wk")).toMatch(/^€1[.,]000\/wk$/);
    });

    it("respects the selected settings currency for market values and wages", function (): void {
        useSettingsStore.setState({
            settings: { ...useSettingsStore.getState().settings, currency: "GBP" },
        });

        expect(formatPlayerMarketValue(125000)).toBe("£125K");
        expect(formatPlayerWage(52000, "/wk")).toMatch(/^£1[.,]000\/wk$/);
    });

    it("maps attribute values to the expected color classes", function (): void {
        expect(getAttributeColorClass(85)).toContain("text-primary-500");
        expect(getAttributeColorClass(65)).toContain("text-accent-600");
        expect(getAttributeColorClass(45)).toContain("text-gray-600");
        expect(getAttributeColorClass(20)).toContain("text-red-500");
    });

    it("resolves injury names for explicit keys and plain injuries", function (): void {
        const translate = (
            key: string,
            options?: { defaultValue?: unknown },
        ): string => {
            return typeof options?.defaultValue === "string"
                ? `${key}:${options.defaultValue}`
                : key;
        };

        expect(resolvePlayerInjuryName("injuries.hamstring", translate)).toBe(
            "injuries.hamstring:injuries.hamstring",
        );
        expect(resolvePlayerInjuryName("Hamstring", translate)).toBe(
            "common.injuries.Hamstring:Hamstring",
        );
    });

    it("builds advanced stats with per-90 values, pass accuracy, and exact-position percentiles", function (): void {
        const player = createPlayer();
        const peers = [
            player,
            createPlayer({
                id: "player-2",
                stats: {
                    ...player.stats,
                    shots: 10,
                    shots_on_target: 5,
                    passes_completed: 70,
                    passes_attempted: 100,
                    tackles_won: 6,
                    interceptions: 4,
                    fouls_committed: 3,
                },
            }),
            createPlayer({
                id: "player-3",
                stats: {
                    ...player.stats,
                    shots: 15,
                    shots_on_target: 8,
                    passes_completed: 75,
                    passes_attempted: 100,
                    tackles_won: 7,
                    interceptions: 5,
                    fouls_committed: 4,
                },
            }),
        ];

        const summary = buildPlayerAdvancedStats(player, peers, {
            minimumMinutes: 180,
            minimumCohortSize: 3,
        });

        expect(summary.percentileEligible).toBe(true);
        expect(summary.metrics.shots.total).toBe(20);
        expect(summary.metrics.shots.per90).toBe(4);
        expect(summary.metrics.shots.percentile).toBe(100);
        expect(summary.metrics.shotsOnTarget.per90).toBe(2);
        expect(summary.metrics.passes.completed).toBe(80);
        expect(summary.metrics.passes.attempted).toBe(100);
        expect(summary.metrics.passes.accuracy).toBe(80);
        expect(summary.metrics.passes.percentile).toBe(100);
        expect(summary.metrics.tacklesWon.per90).toBe(1.8);
        expect(summary.metrics.interceptions.per90).toBe(1.2);
        expect(summary.metrics.foulsCommitted.per90).toBe(1);
    });

    it("hides percentiles when the player is below the minutes threshold or the cohort is too small", function (): void {
        const basePlayer = createPlayer();
        const underThresholdPlayer = createPlayer({
            stats: {
                ...basePlayer.stats,
                minutes_played: 90,
            },
        });

        const underThresholdSummary = buildPlayerAdvancedStats(
            underThresholdPlayer,
            [underThresholdPlayer, createPlayer({ id: "player-2" })],
            {
                minimumMinutes: 180,
                minimumCohortSize: 2,
            },
        );
        const smallCohortSummary = buildPlayerAdvancedStats(
            basePlayer,
            [basePlayer],
            {
                minimumMinutes: 180,
                minimumCohortSize: 2,
            },
        );

        expect(underThresholdSummary.percentileEligible).toBe(false);
        expect(underThresholdSummary.metrics.shots.percentile).toBeNull();
        expect(underThresholdSummary.metrics.passes.percentile).toBeNull();
        expect(smallCohortSummary.metrics.shots.percentile).toBeNull();
        expect(smallCohortSummary.metrics.passes.percentile).toBeNull();
    });
});
