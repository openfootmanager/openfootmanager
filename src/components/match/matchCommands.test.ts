import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { resetMatchCommands, sendMatchCommand } from "./matchCommands";
import type { MatchSnapshot } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const snapshot = (label: string) => ({ label }) as unknown as MatchSnapshot;

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
};

describe("sendMatchCommand", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /**
   * Given two commands sent back to back, the first slow
   * When the first is still in flight
   * Then the second is not sent yet, so the engine sees them in the order they were given and the
   *      snapshots come back in that order, the last one being the second command's.
   */
  it("sends commands one at a time, in order, so the newest snapshot is the last", async () => {
    const first = deferred<MatchSnapshot>();
    const second = deferred<MatchSnapshot>();
    mockedInvoke.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const seen: string[] = [];

    const a = sendMatchCommand({ ChangePlayStyle: { side: "Home", play_style: "A" } }).then((s) =>
      seen.push((s as unknown as { label: string }).label),
    );
    const b = sendMatchCommand({ ChangePlayStyle: { side: "Home", play_style: "B" } }).then((s) =>
      seen.push((s as unknown as { label: string }).label),
    );
    await Promise.resolve();
    expect(mockedInvoke).toHaveBeenCalledTimes(1);

    first.resolve(snapshot("first"));
    await a;
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
    second.resolve(snapshot("second"));
    await b;

    expect(seen).toEqual(["first", "second"]);
  });

  /**
   * Given the first command is refused
   * When a second one is sent behind it
   * Then the second still goes out, and the first's refusal reaches only the first caller.
   */
  it("keeps going after a refused command", async () => {
    mockedInvoke
      .mockRejectedValueOnce("be.error.roleNotValidForPosition")
      .mockResolvedValueOnce(snapshot("second"));

    const first = sendMatchCommand({ ChangePlayerRole: {} });
    const second = sendMatchCommand({ ChangePlayStyle: {} });

    await expect(first).rejects.toBe("be.error.roleNotValidForPosition");
    await expect(second).resolves.toEqual(snapshot("second"));
  });

  /**
   * Given a command in match A whose invoke never answers
   * When a new live session begins for match B
   * Then B's first command goes out at once instead of waiting behind A's.
   */
  it("does not let a hung command in one match block the next match", async () => {
    mockedInvoke.mockReturnValueOnce(new Promise(() => {})).mockResolvedValueOnce(snapshot("b"));
    void sendMatchCommand({ ChangePlayStyle: { side: "Home", play_style: "A" } });

    resetMatchCommands();
    const inMatchB = sendMatchCommand({ ChangePlayStyle: { side: "Home", play_style: "B" } });

    await expect(inMatchB).resolves.toEqual(snapshot("b"));
  });
});
