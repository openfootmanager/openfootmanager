import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export interface BlockerData {
  id: string;
  severity: string;
  text: string;
  tab: string;
}

export interface AdvanceTimeWithModeResponse {
  action: string;
  game?: GameStateData;
  snapshot?: unknown;
  fixture_index?: number;
  mode?: string;
  round_summary?: unknown;
}

export interface SkipToMatchDayResponse {
  action: string;
  game?: GameStateData;
  blockers?: BlockerData[];
  days_skipped?: number;
}

export async function advanceTimeWithMode(
  mode: string,
): Promise<AdvanceTimeWithModeResponse> {
  return invokeCommand<AdvanceTimeWithModeResponse>("advance_time_with_mode", {
    mode,
  });
}

export async function checkBlockingActions(
  logContext: string,
): Promise<BlockerData[]> {
  try {
    const blockers = await invokeCommand<BlockerData[]>("check_blocking_actions");
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
  return invokeCommand<SkipToMatchDayResponse>("skip_to_match_day");
}