import { invoke } from "@tauri-apps/api/core";

import { useGameStore } from "../store/gameStore";

/**
 * Every backend command that changes the active game on the screen's behalf. Each one takes the
 * career token (`careerToken` over IPC) and refuses, inside its write, a request meant for a
 * career other than the active one (`be.error.careerChanged`). This list is the TypeScript half of
 * a contract: `careerInvoke.architecture.test.ts` fails when it differs from the Rust commands
 * that take a `career_token`, and when a service calls one of them with a bare `invoke`.
 */
export const SCREEN_WRITE_COMMANDS = [
  "advance_time",
  "advance_time_with_mode",
  "skip_to_match_day",
  "advance_one_day",
  "advance_to_next_event",
  "save_custom_tactic",
  "delete_custom_tactic",
  "mark_legacy_tactics_imported",
] as const;

export type ScreenWriteCommand = (typeof SCREEN_WRITE_COMMANDS)[number];

/**
 * Calls a screen-write command with the token of the career the screen is showing: the one the
 * game in the store was installed as. A caller that acts later than it was asked (a queued step)
 * passes the token it captured when it was asked, so a request cannot take on a career that
 * replaced its own in between.
 */
export function invokeForCareer<T>(
  command: ScreenWriteCommand,
  args: Record<string, unknown> = {},
  careerToken: number = useGameStore.getState().gameState?.career_token ?? 0,
): Promise<T> {
  return invoke<T>(command, { ...args, careerToken });
}
