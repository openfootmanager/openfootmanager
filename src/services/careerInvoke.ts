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
  "set_formation",
  "apply_tactic_preset",
  "set_starting_xi",
  "set_play_style",
  "set_team_match_roles",
  "set_training",
  "set_training_schedule",
  "set_training_groups",
  "set_player_training_focus",
  "set_player_squad_role",
  "assign_jersey_number",
  "set_team_kit_pattern",
  "set_player_role",
  "set_tactics_phase",
  "toggle_transfer_list",
  "toggle_loan_list",
  "make_transfer_bid",
  "make_loan_offer",
  "exercise_loan_buy_option",
  "respond_to_offer",
  "respond_to_loan_offer",
  "counter_loan_offer",
  "counter_offer",
  "send_scout",
  "start_youth_scouting",
  "cancel_youth_scouting",
  "reassign_youth_scouting",
  "propose_renewal",
  "delegate_renewals",
  "offer_free_agent_contract",
  "set_contract_exit_intent",
  "clear_contract_exit_intent",
  "terminate_contract_now",
  "hire_staff",
  "release_staff",
  "upgrade_facility",
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
