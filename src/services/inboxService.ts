import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, MessageData } from "../store/gameStore";

export async function fetchMessages(): Promise<MessageData[]> {
  return invoke<MessageData[]>("get_messages_page", { query: {} });
}

export interface ResolveMessageActionResult {
  game: GameStateData;
  effect: string | null;
  effect_i18n_key?: string | null;
  effect_i18n_params?: Record<string, string | number> | null;
}

// Inbox-only mutations return just the updated message list (not the whole
// game), so the UI patches its message slice instead of round-tripping the
// entire world on every read/delete.
export async function markMessageRead(
  messageId: string,
): Promise<MessageData[]> {
  return invoke<MessageData[]>("mark_message_read", {
    messageId,
  });
}

export async function resolveMessageAction(
  messageId: string,
  actionId: string,
  optionId?: string | null,
): Promise<ResolveMessageActionResult> {
  return invoke<ResolveMessageActionResult>("resolve_message_action", {
    messageId,
    actionId,
    optionId: optionId ?? null,
  });
}

export async function markAllMessagesRead(): Promise<MessageData[]> {
  return invoke<MessageData[]>("mark_all_messages_read");
}

export async function clearOldMessages(): Promise<MessageData[]> {
  return invoke<MessageData[]>("clear_old_messages");
}

export async function deleteMessage(
  messageId: string,
): Promise<MessageData[]> {
  return invoke<MessageData[]>("delete_message", {
    messageId,
  });
}

export async function deleteMessages(
  messageIds: string[],
): Promise<MessageData[]> {
  return invoke<MessageData[]>("delete_messages", {
    messageIds,
  });
}
