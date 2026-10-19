import type { InvokeArgs } from "@tauri-apps/api/core";

/** A command's arguments, as the page passed them to invoke (always an object in this app). */
export type CommandArgs = Record<string, unknown>;
export type CommandHandlers = Record<string, (args: CommandArgs) => unknown>;

function asCommandArgs(args: InvokeArgs | undefined): CommandArgs {
  return args && !Array.isArray(args) && !ArrayBuffer.isView(args) && !(args instanceof ArrayBuffer)
    ? args
    : {};
}

export interface HarnessIpc {
  /** The mockIPC callback. */
  handle: (cmd: string, args?: InvokeArgs) => unknown;
  /** Every command a page asked for that the harness has no answer to, in the order first asked. */
  unanswered: string[];
}

/**
 * Answers a page's backend calls from the fixture. A command with no handler throws, so the page
 * takes its own error path, and is recorded, so the harness can say which commands a page needs
 * that the fixture does not answer yet instead of rendering a quietly half-empty screen.
 */
export function createHarnessIpc(handlers: CommandHandlers): HarnessIpc {
  const unanswered: string[] = [];
  return {
    unanswered,
    handle(cmd, args) {
      const handler = handlers[cmd];
      if (handler) return handler(asCommandArgs(args));
      if (!unanswered.includes(cmd)) unanswered.push(cmd);
      throw new Error(`page harness: no answer for "${cmd}"`);
    },
  };
}
