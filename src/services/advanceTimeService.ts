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
  home_penalties?: number | null;
  away_penalties?: number | null;
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

/**
 * Roll forward day by day until the next event (user match, blocker, transfer
 * deadline, or high-priority inbox item). Shares the skip response shape.
 */
export async function advanceToNextEvent(): Promise<SkipToMatchDayResponse> {
  return invoke<SkipToMatchDayResponse>("advance_to_next_event");
}

/** Per-day response used by the digest feed loop. */
export interface OneDayResponse {
  /** "advanced" | "match_day" | "blocked" | "fired" */
  action: string;
  game?: GameStateData;
  /** YYYY-MM-DD of the day that was checked or processed. */
  date: string;
  results?: AdvanceMatchResultData[];
  blockers?: BlockerData[];
}

/**
 * Advance exactly one day. Stop conditions (match today, blocker) are checked
 * *before* advancing so the frontend digest loop can surface per-day events
 * without ever auto-simulating the user's own match.
 */
export async function advanceOneDay(): Promise<OneDayResponse> {
  return invoke<OneDayResponse>("advance_one_day");
}