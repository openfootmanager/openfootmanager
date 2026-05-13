import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SubPanel } from "./SubPanel";
import type { EnginePlayerData, EngineTeamData, MatchSnapshot } from "./types";

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, arg?: unknown) => {
            if (typeof arg === "string") {
                return arg;
            }

            if (
                typeof arg === "object" &&
                arg !== null &&
                "used" in arg &&
                "max" in arg
            ) {
                return `${key}:${String((arg as { used: number }).used)}/${String((arg as { max: number }).max)}`;
            }

            if (key === "common.cancel") {
                return "Cancel";
            }

            if (key === "match.selectToTakeOff") {
                return "Select to take off";
            }

            if (key === "match.clearReplacementSelection") {
                return "Clear replacement";
            }

            if (key === "match.selectReplacementMenu") {
                return "Select replacement";
            }

            if (key === "match.selectPlayerToTakeOffFirst") {
                return "Select player to take off first";
            }

            if (key === "match.confirmSubstitution") {
                return "Confirm substitution";
            }

            return key;
        },
    }),
}));

const makePlayer = (overrides: Partial<EnginePlayerData> = {}): EnginePlayerData => ({
    id: "player-1",
    name: "Player One",
    position: "Midfielder",
    condition: 78,
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
    aggression: 60,
    teamwork: 70,
    leadership: 60,
    handling: 20,
    reflexes: 20,
    aerial: 60,
    traits: [],
    ...overrides,
});

const makeTeam = (overrides: Partial<EngineTeamData> = {}): EngineTeamData => ({
    id: "team-1",
    name: "Alpha FC",
    formation: "4-4-2",
    play_style: "Balanced",
    players: [
        makePlayer({ id: "starter-1", name: "Starter One", position: "Midfielder" }),
        makePlayer({ id: "starter-2", name: "Starter Two", position: "Forward", shooting: 80 }),
    ],
    ...overrides,
});

function createSnapshot(): MatchSnapshot {
    return {
        phase: "first_half",
        current_minute: 32,
        home_score: 1,
        away_score: 0,
        possession: "Home",
        ball_zone: "MiddleThird",
        home_team: makeTeam(),
        away_team: makeTeam({
            id: "team-2",
            name: "Beta FC",
            players: [makePlayer({ id: "opp-1", name: "Opponent One" })],
        }),
        home_bench: [
            makePlayer({ id: "bench-1", name: "Bench One", position: "Midfielder", condition: 92 }),
            makePlayer({ id: "bench-2", name: "Bench Two", position: "Forward", shooting: 76 }),
        ],
        away_bench: [makePlayer({ id: "opp-bench-1", name: "Opponent Bench" })],
        home_possession_pct: 56,
        away_possession_pct: 44,
        events: [],
        home_subs_made: 0,
        away_subs_made: 0,
        max_subs: 5,
        home_set_pieces: {
            free_kick_taker: null,
            corner_taker: null,
            penalty_taker: null,
            captain: null,
        },
        away_set_pieces: {
            free_kick_taker: null,
            corner_taker: null,
            penalty_taker: null,
            captain: null,
        },
        substitutions: [],
        allows_extra_time: false,
        home_yellows: {},
        away_yellows: {},
        sent_off: [],
    };
}

describe("SubPanel", () => {
    it("shows a disabled bench context menu action until a player is selected to come off", () => {
        render(
            <SubPanel
                snapshot={createSnapshot()}
                side="Home"
                onSubstitute={vi.fn()}
                onClose={vi.fn()}
            />,
        );

        fireEvent.contextMenu(screen.getByTestId("sub-panel-bench-bench-1"));

        expect(
            screen.getByRole("button", { name: "Select player to take off first" }),
        ).toBeDisabled();
    });

    it("supports the substitution selection flow through context menus", () => {
        const onSubstitute = vi.fn();

        render(
            <SubPanel
                snapshot={createSnapshot()}
                side="Home"
                onSubstitute={onSubstitute}
                onClose={vi.fn()}
            />,
        );

        fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));
        fireEvent.click(screen.getByRole("button", { name: "Select to take off" }));

        fireEvent.contextMenu(screen.getByTestId("sub-panel-bench-bench-1"));
        fireEvent.click(screen.getByRole("button", { name: "Select replacement" }));

        fireEvent.click(
            screen.getByRole("button", { name: "Confirm substitution" }),
        );

        expect(onSubstitute).toHaveBeenCalledWith("starter-1", "bench-1");
    });

    it("allows clearing the selected off-player through the context menu", () => {
        render(
            <SubPanel
                snapshot={createSnapshot()}
                side="Home"
                onSubstitute={vi.fn()}
                onClose={vi.fn()}
            />,
        );

        fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));
        fireEvent.click(screen.getByRole("button", { name: "Select to take off" }));

        expect(screen.getByText("match.selectBenchToCompare")).toBeInTheDocument();

        fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));
        fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

        expect(screen.queryByText("match.selectBenchToCompare")).not.toBeInTheDocument();
    });
});