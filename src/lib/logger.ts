import { invoke } from "@tauri-apps/api/core";

/**
 * Writes a message into the log file the Rust side writes (tauri-plugin-log: stdout and the log
 * directory; a file is rotated at 5 MB and the old ones are kept).
 *
 * Until this existed nothing the frontend knew reached disk, so the log folder a bug report carries
 * described the backend and was silent about the render that broke. The plugin's JS package is not a
 * dependency of this release; its `log` command is called directly (level 5 is "error").
 *
 * **Nothing here may throw or reject.** The call rejects whenever the plugin is not there (outside
 * Tauri, or a missing capability), and a logger that fails must never be worse than the bug it
 * reports: the failure goes to the console instead.
 */
const ERROR_LEVEL = 5;

export function logError(message: string): void {
  try {
    void invoke("plugin:log|log", { level: ERROR_LEVEL, message }).catch(() => {
      console.warn("Could not write to the log file:", message);
    });
  } catch {
    console.warn("Could not write to the log file:", message);
  }
}
