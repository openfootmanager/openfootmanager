import { describe, expect, it } from "vitest";

import { createHarnessIpc } from "./harnessIpc";

describe("createHarnessIpc", () => {
  /** Given a handler for a command, when the page invokes it, then it gets the handler's answer for its arguments. */
  it("answers a command from its handler", () => {
    const ipc = createHarnessIpc({ get_squad: (args) => [`squad of ${String(args.teamId)}`] });
    expect(ipc.handle("get_squad", { teamId: "club-1" })).toEqual(["squad of club-1"]);
    expect(ipc.unanswered).toEqual([]);
  });

  /**
   * Given no handler for a command, when the page invokes it twice,
   * then each call fails naming the command, and the command is recorded once.
   */
  it("fails a command it has no answer to, and records it once", () => {
    const ipc = createHarnessIpc({});
    expect(() => ipc.handle("get_inbox")).toThrow('no answer for "get_inbox"');
    expect(() => ipc.handle("get_inbox")).toThrow();
    expect(ipc.unanswered).toEqual(["get_inbox"]);
  });
});
