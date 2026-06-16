import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";

export interface BlockerData {
  id: string;
  severity: string;
  text: string;
  text_key?: string;
  text_params?: Record<string, string>;
  tab: string;
}

/** One finished match shown in the post-advance results recap. */
export interface AdvanceMatchResultData {
  date: string;
  competition: string;
  international: boolean;
  home_team: string;
  away_team: string;
  home_goals: number;
  away_goals: number;
  involves_user: boolean;
}

export interface AdvanceTimeWithModeResponse {
  action: string;
  game?: GameStateData;
  snapshot?: unknown;
  fixture_index?: number;
  mode?: string;
  round_summary?: unknown;
  results?: AdvanceMatchResultData[];
}

export interface SkipToMatchDayResponse {
  action: string;
  game?: GameStateData;
  blockers?: BlockerData[];
  days_skipped?: number;
  results?: AdvanceMatchResultData[];
}

export async function advanceTimeWithMode(
  mode: string,
): Promise<AdvanceTimeWithModeResponse> {
  return invoke<AdvanceTimeWithModeResponse>("advance_time_with_mode", {
    mode,
  });
}

export async function checkBlockingActions(
  logContext: string,
): Promise<BlockerData[]> {
  try {
    const blockers = await invoke<BlockerData[]>("check_blocking_actions");
    console.info(`[useAdvanceTime] ${logContext}:blockers`, {
      count: blockers.length,
      blockers,
    });
    return blockers;
  } catch (err) {
    console.warn(`[useAdvanceTime] ${logContext}:blockerCheckFailed`, err);
    return [];
  }
}

export async function skipToMatchDay(): Promise<SkipToMatchDayResponse> {
  return invoke<SkipToMatchDayResponse>("skip_to_match_day");
}