import { message } from "@tauri-apps/plugin-dialog";

import { logError } from "./logger";

/**
 * Show the user an error, and record it.
 *
 * This replaces `window.alert`, which was the app's only way of reporting a failure and was a bad
 * one twice over. In the WebKitGTK webview it is not a reliable way to put anything in front of a
 * player at all, and even where it does render it blocks the whole webview until dismissed. Worse,
 * an `alert` leaves nothing behind: the failure it reported existed only on screen, so a player
 * who dismissed it had nothing left to send us.
 *
 * The log write happens **first and unconditionally**. If the dialog is the unreliable half, the
 * log line is what survives — and a bug report without it is a report about a message nobody can
 * reproduce. The dialog failing must never cost us the record of what failed.
 */
export async function showError(title: string, detail: string): Promise<void> {
  logError(`[ui] ${title}: ${detail}`);
  try {
    await message(detail, { title, kind: "error" });
  } catch {
    // No dialog plugin (a browser, a revoked capability, a webview mid-teardown). The line above
    // already recorded it; losing the dialog must not also lose the error.
  }
}
