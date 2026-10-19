import type { InvokeArgs } from "@tauri-apps/api/core";

/** A command's arguments, as the page passed them to invoke (always an object in this app). */
export type CommandArgs = Record<string, unknown>;
export type CommandHandlers = Record<string, (args: CommandArgs) => unknown>;

/** What a handler returns for arguments it has no answer to, such as another club's id. */
export const UNANSWERED = Symbol("unanswered");

function asCommandArgs(args: InvokeArgs | undefined): CommandArgs {
  return args && !Array.isArray(args) && !ArrayBuffer.isView(args) && !(args instanceof ArrayBuffer)
    ? args
    : {};
}

export interface HarnessIpc {
  /** The mockIPC callback. */
  handle: (cmd: string, args?: InvokeArgs) => unknown;
  /**
   * Every call the harness has no answer to, in the order first asked: a command with no handler
   * by name, and a call its handler declined with its arguments.
   */
  unanswered: string[];
}

/**
 * Answers a page's backend calls from the fixture. A command with no handler throws, so the page
 * takes its own error path, and is recorded, so the harness can say which commands a page needs
 * that the fixture does not answer yet instead of rendering a quietly half-empty screen.
 */
export function createHarnessIpc(handlers: CommandHandlers): HarnessIpc {
  const unanswered: string[] = [];
  function decline(cmd: string, call: string): never {
    if (!unanswered.includes(call)) unanswered.push(call);
    throw new Error(`page harness: no answer for "${cmd}"`);
  }
  return {
    unanswered,
    handle(cmd, args) {
      if (!(cmd in handlers)) decline(cmd, cmd);
      const callArgs = asCommandArgs(args);
      const answer = handlers[cmd](callArgs);
      if (answer === UNANSWERED) decline(cmd, `${cmd} ${JSON.stringify(callArgs)}`);
      return answer;
    },
  };
}
