import type { MatchSnapshot, RoundSummary } from "../components/match/types";
import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export interface StartLiveMatchOptions {
    fixtureIndex: number;
    mode: string;
    allowsExtraTime: boolean;
}

export interface FinishLiveMatchResponse {
    game: GameStateData;
    round_summary?: RoundSummary | null;
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