import type {
    MatchSnapshot,
    MinuteResult,
    RoundSummary,
    TeamTalkTone,
} from "../components/match/types";
import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

type MatchSide = "Home" | "Away";
type TeamTalkContext = "winning" | "losing" | "drawing";
export type SetPieceRole = "captain" | "penalty" | "freekick" | "corner";

const SET_PIECE_COMMANDS: Record<SetPieceRole, string> = {
    captain: "SetCaptain",
    corner: "SetCornerTaker",
    freekick: "SetFreeKickTaker",
    penalty: "SetPenaltyTaker",
};

export interface StartLiveMatchOptions {
    fixtureIndex: number;
    mode: string;
    allowsExtraTime: boolean;
}

export interface FinishLiveMatchResponse {
    game: GameStateData;
    round_summary?: RoundSummary | null;
}

export interface TeamTalkResult {
    player_id: string;
    player_name: string;
    old_morale: number;
    new_morale: number;
    delta: number;
}

export interface AutoSelectedSetPieces {
    captain: string | null;
    penalty_taker: string | null;
    free_kick_taker: string | null;
    corner_taker: string | null;
}

export interface PressConferenceAnswerPayload {
    question_id: string;
    response_id: string;
    response_tone: string;
    response_text: string;
    question_text: string;
    player_id?: string;
}

export interface SubmitPressConferenceOptions {
    answers: PressConferenceAnswerPayload[];
    homeTeam: string;
    awayTeam: string;
    homeScore: number;
    awayScore: number;
    userTeamName: string;
    userTeamId: string;
    prerenderedBody: string;
    prerenderedHeadline: string;
}

export interface SubmitPressConferenceResponse {
    game: GameStateData;
    morale_delta: number;
}

async function applyMatchCommand(command: object): Promise<MatchSnapshot> {
    return invokeCommand<MatchSnapshot>("apply_match_command", { command });
}

export async function getMatchSnapshot(): Promise<MatchSnapshot> {
    return invokeCommand<MatchSnapshot>("get_match_snapshot");
}

export async function startLiveMatchSession(
    options: StartLiveMatchOptions,
): Promise<MatchSnapshot> {
    return invokeCommand<MatchSnapshot>("start_live_match", {
        allowsExtraTime: options.allowsExtraTime,
        fixtureIndex: options.fixtureIndex,
        mode: options.mode,
    });
}

export async function finishLiveMatch(): Promise<FinishLiveMatchResponse> {
    return invokeCommand<FinishLiveMatchResponse>("finish_live_match");
}

export async function stepLiveMatch(minutes = 1): Promise<MinuteResult[]> {
    return invokeCommand<MinuteResult[]>("step_live_match", { minutes });
}

export async function changeMatchFormation(
    side: MatchSide,
    formation: string,
): Promise<MatchSnapshot> {
    return applyMatchCommand({ ChangeFormation: { side, formation } });
}

export async function changeMatchPlayStyle(
    side: MatchSide,
    playStyle: string,
): Promise<MatchSnapshot> {
    return applyMatchCommand({
        ChangePlayStyle: { side, play_style: playStyle },
    });
}

export async function swapPreMatchPlayers(
    side: MatchSide,
    playerOffId: string,
    playerOnId: string,
): Promise<MatchSnapshot> {
    return applyMatchCommand({
        PreMatchSwap: {
            side,
            player_off_id: playerOffId,
            player_on_id: playerOnId,
        },
    });
}

export async function substitutePlayer(
    side: MatchSide,
    playerOffId: string,
    playerOnId: string,
): Promise<MatchSnapshot> {
    return applyMatchCommand({
        Substitute: {
            side,
            player_off_id: playerOffId,
            player_on_id: playerOnId,
        },
    });
}

export async function setMatchSetPieceTaker(
    side: MatchSide,
    role: SetPieceRole,
    playerId: string,
): Promise<MatchSnapshot> {
    return applyMatchCommand({
        [SET_PIECE_COMMANDS[role]]: { side, player_id: playerId },
    });
}

export async function autoSelectSetPieces(
    playerIds: string[],
): Promise<AutoSelectedSetPieces> {
    return invokeCommand<AutoSelectedSetPieces>("auto_select_set_pieces", {
        playerIds,
    });
}

export async function applyTeamTalk(
    tone: TeamTalkTone,
    context: TeamTalkContext,
): Promise<TeamTalkResult[]> {
    return invokeCommand<TeamTalkResult[]>("apply_team_talk", {
        context,
        tone,
    });
}

export async function submitPressConference(
    options: SubmitPressConferenceOptions,
): Promise<SubmitPressConferenceResponse> {
    return invokeCommand<SubmitPressConferenceResponse>(
        "submit_press_conference",
        {
            answers: options.answers,
            awayScore: options.awayScore,
            awayTeam: options.awayTeam,
            homeScore: options.homeScore,
            homeTeam: options.homeTeam,
            prerenderedBody: options.prerenderedBody,
            prerenderedHeadline: options.prerenderedHeadline,
            userTeamId: options.userTeamId,
            userTeamName: options.userTeamName,
        },
    );
}