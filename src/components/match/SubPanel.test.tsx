import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { getPositionOvr } from "./PreMatchLineup";
import { SubPanel } from "./SubPanel";
import type { EnginePlayerData, EngineTeamData, MatchSnapshot } from "./types";

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, opts?: Record<string, unknown> | string) => {
            if (typeof opts === "string") {
                return opts;
            }

            if (opts && "used" in opts && "max" in opts) {
                return `${key}:${String(opts.used)}/${String(opts.max)}`;
            }

            if (opts && "name" in opts) {
                return `${key}:${String(opts.name)}`;
            }

            return key;
        },
    }),
}));

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

const makeTeam = (overrides: Partial<EngineTeamData> = {}): EngineTeamData => ({
    id: "team-1",
    name: "Test FC",
    formation: "4-4-2",
    play_style: "Balanced",
    players: [],
    ...overrides,
});

const makeSnapshot = (overrides: Partial<MatchSnapshot> = {}): MatchSnapshot => ({
    phase: "SecondHalf",
    current_minute: 62,
    home_score: 1,
    away_score: 0,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: makeTeam(),
    away_team: makeTeam({ id: "team-2", name: "Away FC" }),
    home_bench: [],
    away_bench: [],
    home_possession_pct: 55,
    away_possession_pct: 45,
    events: [],
    home_subs_made: 0,
    away_subs_made: 0,
    max_subs: 5,
    home_set_pieces: { free_kick_taker: null, corner_taker: null, penalty_taker: null, captain: null },
    away_set_pieces: { free_kick_taker: null, corner_taker: null, penalty_taker: null, captain: null },
    substitutions: [],
    allows_extra_time: false,
    home_yellows: {},
    away_yellows: {},
    sent_off: [],
    ...overrides,
});

describe("SubPanel", () => {
    it("renders shared OVR values for starters and bench players", () => {
        const starter = makePlayer({
            id: "starter-1",
            name: "Starter Mid",
            position: "Midfielder",
            passing: 80,
            vision: 75,
            decisions: 70,
            stamina: 65,
            dribbling: 60,
            teamwork: 55,
        });
        const bench = makePlayer({
            id: "bench-1",
            name: "Bench Utility",
            position: "Sweeper",
            pace: 66,
            stamina: 64,
            strength: 68,
            passing: 60,
            shooting: 40,
            tackling: 72,
            dribbling: 58,
            defending: 74,
            positioning: 76,
            vision: 54,
            decisions: 52,
        });
        const snapshot = makeSnapshot({
            home_team: makeTeam({ players: [starter] }),
            home_bench: [bench],
        });

        render(
            <SubPanel
                snapshot={snapshot}
                side="Home"
                onSubstitute={vi.fn()}
                onClose={vi.fn()}
            />,
        );

        const starterRow = screen.getByText("Starter Mid").closest("tr");
        const benchRow = screen.getByText("Bench Utility").closest("tr");

        expect(starterRow).not.toBeNull();
        expect(benchRow).not.toBeNull();
        expect(within(starterRow!).getByText(String(getPositionOvr(starter)))).toBeInTheDocument();
        expect(within(benchRow!).getByText(String(getPositionOvr(bench)))).toBeInTheDocument();
    });

    it("confirms a substitution after selecting an on-field and bench player", () => {
        const onSubstitute = vi.fn();
        const starter = makePlayer({ id: "starter-1", name: "Starter Mid", position: "Midfielder" });
        const bench = makePlayer({ id: "bench-1", name: "Bench Mid", position: "Midfielder" });
        const snapshot = makeSnapshot({
            home_team: makeTeam({ players: [starter] }),
            home_bench: [bench],
        });

        render(
            <SubPanel
                snapshot={snapshot}
                side="Home"
                onSubstitute={onSubstitute}
                onClose={vi.fn()}
            />,
        );

        fireEvent.click(screen.getByText("Starter Mid"));
        fireEvent.click(screen.getByText("Bench Mid"));
        fireEvent.click(screen.getByText("match.confirmSubstitution"));

        expect(onSubstitute).toHaveBeenCalledWith("starter-1", "bench-1");
    });
});