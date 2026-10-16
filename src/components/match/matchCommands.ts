import { invoke } from "@tauri-apps/api/core";

import type { MatchSnapshot } from "./types";

// Tauri does not order independent invokes, so two quick changes could be answered newest first and
// the older snapshot would overwrite the newer. Every in-play command goes through one chain, so
// the engine receives them, and the screen hears their snapshots, in the order they were given.
let chain: Promise<unknown> = Promise.resolve();

/**
 * Starts a fresh chain for a new live session. One command whose invoke never answers would
 * otherwise hold up every later command, in this match and the next, until the app reloads.
 */
export function resetMatchCommands(): void {
  chain = Promise.resolve();
}

/**
 * Sends one command to the live match once every earlier one has been answered. A refused command
 * rejects only its own caller; the commands behind it still go out.
 */
export function sendMatchCommand(command: Record<string, unknown>): Promise<MatchSnapshot> {
  const send = () => invoke<MatchSnapshot>("apply_match_command", { command });
  const result = chain.then(send, send);
  chain = result.catch(() => undefined);
  return result;
}
