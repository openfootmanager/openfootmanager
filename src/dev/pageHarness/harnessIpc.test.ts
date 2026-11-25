import { describe, expect, it } from "vitest";

import { createHarnessIpc, UNANSWERED } from "./harnessIpc";

describe("a handler that declines", () => {
  /**
   * Given a handler that only knows the user's club,
   * when the page asks it about another club,
   * then the call fails and is recorded with its arguments, instead of quietly answering with the
   * user's club.
   */
  it("fails and records a call it has no answer for", () => {
    const ipc = createHarnessIpc({
      get_squad: (args) => (args.teamId === "club-1" ? ["squad"] : UNANSWERED),
    });
    expect(ipc.handle("get_squad", { teamId: "club-1" })).toEqual(["squad"]);
    expect(() => ipc.handle("get_squad", { teamId: "club-2" })).toThrow(
      'no answer for "get_squad"',
    );
    expect(ipc.unanswered).toEqual(['get_squad {"teamId":"club-2"}']);
  });
});

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

describe("the commands a page asked for", () => {
  /**
   * Given a page that asks for a command it has an answer to, one it has none to, and the first
   *       again
   * When the harness is asked what the page asked for
   * Then every call is listed in order, answered or not, so a test can say a page did not ask
   *      for a command yet
   */
  it("lists every command asked, in order, answered or not", () => {
    const ipc = createHarnessIpc({ get_squad: () => ["squad"] });

    ipc.handle("get_squad", { teamId: "club-1" });
    expect(() => ipc.handle("get_inbox")).toThrow();
    ipc.handle("get_squad", { teamId: "club-1" });

    expect(ipc.calls).toEqual(["get_squad", "get_inbox", "get_squad"]);
  });
});
