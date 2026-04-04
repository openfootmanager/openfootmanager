import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export interface ResolveMessageActionResult {
  game: GameStateData;
  effect: string | null;
  effect_i18n_key?: string | null;
  effect_i18n_params?: Record<string, string | number> | null;
}

export async function markMessageRead(
  messageId: string,
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("mark_message_read", {
    messageId,
  });
}

export async function resolveMessageAction(
  messageId: string,
  actionId: string,
  optionId?: string | null,
): Promise<ResolveMessageActionResult> {
  return invokeCommand<ResolveMessageActionResult>("resolve_message_action", {
    messageId,
    actionId,
    optionId: optionId ?? null,
  });
}

export async function markAllMessagesRead(): Promise<GameStateData> {
  return invokeCommand<GameStateData>("mark_all_messages_read");
}

export async function clearOldMessages(): Promise<GameStateData> {
  return invokeCommand<GameStateData>("clear_old_messages");
}

export async function deleteMessage(
  messageId: string,
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("delete_message", {
    messageId,
  });
}

export async function deleteMessages(
  messageIds: string[],
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("delete_messages", {
    messageIds,
  });
}