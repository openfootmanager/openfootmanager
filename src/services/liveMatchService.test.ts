import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
    applyTeamTalk,
    autoSelectSetPieces,
    changeMatchFormation,
    changeMatchPlayStyle,
    finishLiveMatch,
    getMatchSnapshot,
    setMatchSetPieceTaker,
    startLiveMatchSession,
    stepLiveMatch,
    substitutePlayer,
    submitPressConference,
    swapPreMatchPlayers,
} from "./liveMatchService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("liveMatchService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("loads the current live match snapshot", async () => {
        const snapshot = { phase: "FirstHalf" };
        mockedInvoke.mockResolvedValueOnce(snapshot);

        await expect(getMatchSnapshot()).resolves.toBe(snapshot);
        expect(mockedInvoke).toHaveBeenCalledWith("get_match_snapshot");
    });

    it("starts or restores a live match session", async () => {
        const snapshot = { phase: "PreMatch" };
        mockedInvoke.mockResolvedValueOnce(snapshot);

        await expect(
            startLiveMatchSession({
                allowsExtraTime: false,
                fixtureIndex: 7,
                mode: "spectator",
            }),
        ).resolves.toBe(snapshot);

        expect(mockedInvoke).toHaveBeenCalledWith("start_live_match", {
            allowsExtraTime: false,
            fixtureIndex: 7,
            mode: "spectator",
        });
    });

    it("finalizes the live match", async () => {
        const response = { game: { manager: { id: "manager-1" } } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(finishLiveMatch()).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("finish_live_match");
    });

    it("steps the live match clock", async () => {
        const results = [{ phase: "FirstHalf", events: [] }];
        mockedInvoke.mockResolvedValueOnce(results);

        await expect(stepLiveMatch(3)).resolves.toBe(results);
        expect(mockedInvoke).toHaveBeenCalledWith("step_live_match", {
            minutes: 3,
        });
    });

    it("applies formation and play style match commands", async () => {
        const snapshot = { phase: "FirstHalf" };
        mockedInvoke.mockResolvedValue(snapshot);

        await expect(changeMatchFormation("Home", "4-3-3")).resolves.toBe(
            snapshot,
        );
        expect(mockedInvoke).toHaveBeenCalledWith("apply_match_command", {
            command: { ChangeFormation: { side: "Home", formation: "4-3-3" } },
        });

        await expect(
            changeMatchPlayStyle("Away", "Counter"),
        ).resolves.toBe(snapshot);
        expect(mockedInvoke).toHaveBeenCalledWith("apply_match_command", {
            command: {
                ChangePlayStyle: { side: "Away", play_style: "Counter" },
            },
        });
    });

    it("applies lineup and substitution commands", async () => {
        const snapshot = { phase: "PreMatch" };
        mockedInvoke.mockResolvedValue(snapshot);

        await expect(
            swapPreMatchPlayers("Home", "player-off", "player-on"),
        ).resolves.toBe(snapshot);
        expect(mockedInvoke).toHaveBeenCalledWith("apply_match_command", {
            command: {
                PreMatchSwap: {
                    side: "Home",
                    player_off_id: "player-off",
                    player_on_id: "player-on",
                },
            },
        });

        await expect(
            substitutePlayer("Away", "starter", "bench"),
        ).resolves.toBe(snapshot);
        expect(mockedInvoke).toHaveBeenCalledWith("apply_match_command", {
            command: {
                Substitute: {
                    side: "Away",
                    player_off_id: "starter",
                    player_on_id: "bench",
                },
            },
        });
    });

    it("updates set-piece roles and auto-selects takers", async () => {
        const snapshot = { phase: "PreMatch" };
        mockedInvoke.mockResolvedValueOnce(snapshot).mockResolvedValueOnce({
            captain: "captain-id",
            penalty_taker: "penalty-id",
            free_kick_taker: null,
            corner_taker: "corner-id",
        });

        await expect(
            setMatchSetPieceTaker("Home", "captain", "captain-id"),
        ).resolves.toBe(snapshot);
        expect(mockedInvoke).toHaveBeenCalledWith("apply_match_command", {
            command: {
                SetCaptain: { side: "Home", player_id: "captain-id" },
            },
        });

        await expect(autoSelectSetPieces(["a", "b"])).resolves.toEqual({
            captain: "captain-id",
            penalty_taker: "penalty-id",
            free_kick_taker: null,
            corner_taker: "corner-id",
        });
        expect(mockedInvoke).toHaveBeenCalledWith("auto_select_set_pieces", {
            playerIds: ["a", "b"],
        });
    });

    it("submits team talks and press conference answers", async () => {
        const talkResult = [
            {
                player_id: "player-1",
                player_name: "Player One",
                old_morale: 60,
                new_morale: 65,
                delta: 5,
            },
        ];
        const pressConferenceResult = {
            game: { manager: { id: "manager-1" } },
            morale_delta: 3,
        };
        mockedInvoke
            .mockResolvedValueOnce(talkResult)
            .mockResolvedValueOnce(pressConferenceResult);

        await expect(applyTeamTalk("praise", "winning")).resolves.toBe(
            talkResult,
        );
        expect(mockedInvoke).toHaveBeenCalledWith("apply_team_talk", {
            context: "winning",
            tone: "praise",
        });

        await expect(
            submitPressConference({
                answers: [
                    {
                        question_id: "q1",
                        response_id: "r1",
                        response_tone: "calm",
                        response_text: "We played well.",
                        question_text: "How do you feel?",
                        player_id: "player-1",
                    },
                ],
                awayScore: 1,
                awayTeam: "Away FC",
                homeScore: 2,
                homeTeam: "Home FC",
                prerenderedBody: "Body",
                prerenderedHeadline: "Headline",
                userTeamId: "home-1",
                userTeamName: "Home FC",
            }),
        ).resolves.toBe(pressConferenceResult);
        expect(mockedInvoke).toHaveBeenCalledWith("submit_press_conference", {
            answers: [
                {
                    question_id: "q1",
                    response_id: "r1",
                    response_tone: "calm",
                    response_text: "We played well.",
                    question_text: "How do you feel?",
                    player_id: "player-1",
                },
            ],
            awayScore: 1,
            awayTeam: "Away FC",
            homeScore: 2,
            homeTeam: "Home FC",
            prerenderedBody: "Body",
            prerenderedHeadline: "Headline",
            userTeamId: "home-1",
            userTeamName: "Home FC",
        });
    });
});