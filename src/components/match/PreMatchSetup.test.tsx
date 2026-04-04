import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PreMatchSetup from "./PreMatchSetup";
import type { EnginePlayerData, EngineTeamData, MatchSnapshot } from "./types";

const autoSelectSetPiecesMock = vi.fn();
const changeMatchFormationMock = vi.fn();
const changeMatchPlayStyleMock = vi.fn();
const setMatchSetPieceTakerMock = vi.fn();
const swapPreMatchPlayersMock = vi.fn();

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, opts?: Record<string, unknown> | string) => {
            if (typeof opts === "string") {
                return opts;
            }

            if (opts && "count" in opts) {
                return `${key}:${String(opts.count)}`;
            }

            return key;
        },
    }),
}));

vi.mock("./MatchScreenLayout", () => ({
    default: ({ header, children }: { header: React.ReactNode; children: React.ReactNode }) => (
        <div>
            <div>{header}</div>
            <div>{children}</div>
        </div>
    ),
}));

vi.mock("./SetPieceSelector", () => ({
    default: ({ label }: { label: string }) => <div>{label}</div>,
}));

vi.mock("../../services/liveMatchService", () => ({
    autoSelectSetPieces: (...args: unknown[]) => autoSelectSetPiecesMock(...args),
    changeMatchFormation: (...args: unknown[]) => changeMatchFormationMock(...args),
    changeMatchPlayStyle: (...args: unknown[]) => changeMatchPlayStyleMock(...args),
    setMatchSetPieceTaker: (...args: unknown[]) => setMatchSetPieceTakerMock(...args),
    swapPreMatchPlayers: (...args: unknown[]) => swapPreMatchPlayersMock(...args),
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
    phase: "PreKickOff",
    current_minute: 0,
    home_score: 0,
    away_score: 0,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: makeTeam({ id: "home-1", name: "Home FC" }),
    away_team: makeTeam({ id: "away-1", name: "Away FC" }),
    home_bench: [],
    away_bench: [],
    home_possession_pct: 50,
    away_possession_pct: 50,
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

const makeGameState = (players: EnginePlayerData[]) => ({
    teams: [
        {
            id: "home-1",
            colors: { primary: "#00ff00" },
        },
        {
            id: "away-1",
            colors: { primary: "#ff0000" },
        },
    ],
    players: players.map((player) => ({
        ...player,
        team_id: "home-1",
    })),
}) as never;

describe("PreMatchSetup", () => {
    beforeEach(() => {
        autoSelectSetPiecesMock.mockReset();
        changeMatchFormationMock.mockReset();
        changeMatchPlayStyleMock.mockReset();
        setMatchSetPieceTakerMock.mockReset();
        swapPreMatchPlayersMock.mockReset();
    });

    it("applies the planned auto-select swaps and reports the final snapshot", async () => {
        const starterGoalkeeper = makePlayer({ id: "gk-1", name: "Keeper", position: "Goalkeeper", handling: 80, reflexes: 80 });
        const starterDefender = makePlayer({ id: "d-weak", name: "Weak Defender", position: "Defender", condition: 55, defending: 50, tackling: 50, positioning: 50, strength: 50 });
        const starterDefenderTwo = makePlayer({ id: "d-2", name: "Defender Two", position: "Defender", defending: 76, tackling: 75, positioning: 74, strength: 73 });
        const starterDefenderThree = makePlayer({ id: "d-3", name: "Defender Three", position: "Defender", defending: 74, tackling: 74, positioning: 73, strength: 72 });
        const starterDefenderFour = makePlayer({ id: "d-4", name: "Defender Four", position: "Defender", defending: 72, tackling: 71, positioning: 70, strength: 69 });
        const starterMidfielder = makePlayer({ id: "m-1", name: "Midfielder", position: "Midfielder", passing: 80, vision: 78, decisions: 76 });
        const starterMidfielderTwo = makePlayer({ id: "m-2", name: "Midfielder Two", position: "Midfielder", passing: 78, vision: 76, decisions: 74 });
        const starterMidfielderThree = makePlayer({ id: "m-3", name: "Midfielder Three", position: "Midfielder", passing: 76, vision: 74, decisions: 72 });
        const starterMidfielderFour = makePlayer({ id: "m-4", name: "Midfielder Four", position: "Midfielder", passing: 74, vision: 72, decisions: 70 });
        const starterForward = makePlayer({ id: "f-1", name: "Forward", position: "Forward", shooting: 82, positioning: 79, pace: 77 });
        const starterForwardTwo = makePlayer({ id: "f-2", name: "Forward Two", position: "Forward", shooting: 80, positioning: 77, pace: 75 });
        const benchDefender = makePlayer({ id: "d-strong", name: "Strong Defender", position: "Defender", condition: 95, defending: 82, tackling: 81, positioning: 80, strength: 79 });

        const initialSnapshot = makeSnapshot({
            home_team: makeTeam({
                id: "home-1",
                name: "Home FC",
                formation: "4-4-2",
                players: [
                    starterGoalkeeper,
                    starterDefender,
                    starterDefenderTwo,
                    starterDefenderThree,
                    starterDefenderFour,
                    starterMidfielder,
                    starterMidfielderTwo,
                    starterMidfielderThree,
                    starterMidfielderFour,
                    starterForward,
                    starterForwardTwo,
                ],
            }),
            home_bench: [benchDefender],
        });
        const updatedSnapshot = makeSnapshot({
            home_team: makeTeam({
                id: "home-1",
                name: "Home FC",
                formation: "4-4-2",
                players: [
                    starterGoalkeeper,
                    benchDefender,
                    starterDefenderTwo,
                    starterDefenderThree,
                    starterDefenderFour,
                    starterMidfielder,
                    starterMidfielderTwo,
                    starterMidfielderThree,
                    starterMidfielderFour,
                    starterForward,
                    starterForwardTwo,
                ],
            }),
            home_bench: [starterDefender],
        });

        swapPreMatchPlayersMock.mockResolvedValue(updatedSnapshot);
        const onUpdateSnapshot = vi.fn();

        render(
            <PreMatchSetup
                snapshot={initialSnapshot}
                gameState={makeGameState([
                    starterGoalkeeper,
                    starterDefender,
                    starterDefenderTwo,
                    starterDefenderThree,
                    starterDefenderFour,
                    starterMidfielder,
                    starterMidfielderTwo,
                    starterMidfielderThree,
                    starterMidfielderFour,
                    starterForward,
                    starterForwardTwo,
                    benchDefender,
                ])}
                userSide="Home"
                onStart={vi.fn()}
                onUpdateSnapshot={onUpdateSnapshot}
            />,
        );

        fireEvent.click(screen.getByText("match.autoSelectXI"));

        await waitFor(() => {
            expect(swapPreMatchPlayersMock).toHaveBeenCalledWith(
                "Home",
                "d-weak",
                "d-strong",
            );
        });
        expect(onUpdateSnapshot).toHaveBeenCalledWith(updatedSnapshot);
    });
});