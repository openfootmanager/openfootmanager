import { invoke } from "@tauri-apps/api/core";

import type { MatchSnapshot, MinuteResult } from "../store/liveMatchTypes";
import { invokeForCareer } from "./careerInvoke";

/**
 * Plays the live match on by `minutes`. In a penalty shootout a "minute" is one kick. The engine
 * stops a batch early on entering a phase that needs the manager, so a half time, a shootout or the
 * final whistle is always the last result. It writes the game, so it carries the career token.
 */
export async function stepLiveMatch(minutes: number): Promise<MinuteResult[]> {
  return invokeForCareer<MinuteResult[]>("step_live_match", { minutes });
}

/** The live match as it stands: teams, score, events and, in a shootout, its tally. */
export async function getMatchSnapshot(): Promise<MatchSnapshot> {
  return invoke<MatchSnapshot>("get_match_snapshot");
}
