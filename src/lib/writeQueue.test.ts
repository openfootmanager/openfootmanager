import { describe, it, expect } from "vitest";

import { createWriteQueue } from "./writeQueue";

const tick = (ms: number) => new Promise((r) => setTimeout(r, ms));

describe("createWriteQueue", () => {
  it("runs tasks one at a time, never overlapping", async () => {
    const enqueue = createWriteQueue();
    const events: string[] = [];
    let active = 0;

    const make = (id: string, delay: number) => async () => {
      active += 1;
      expect(active).toBe(1); // no two tasks run concurrently
      events.push(`start-${id}`);
      await tick(delay);
      events.push(`end-${id}`);
      active -= 1;
    };

    // Enqueue out of "natural" finishing order: the first is slowest.
    const a = enqueue(make("a", 30));
    const b = enqueue(make("b", 5));
    const c = enqueue(make("c", 5));
    await Promise.all([a, b, c]);

    expect(events).toEqual([
      "start-a", "end-a",
      "start-b", "end-b",
      "start-c", "end-c",
    ]);
  });

  it("keeps the queue running after a task rejects", async () => {
    const enqueue = createWriteQueue();
    const order: string[] = [];

    const failing = enqueue(async () => {
      order.push("fail");
      throw new Error("boom");
    });
    const next = enqueue(async () => {
      order.push("next");
      return "ok";
    });

    await expect(failing).rejects.toThrow("boom");
    await expect(next).resolves.toBe("ok");
    expect(order).toEqual(["fail", "next"]);
  });

  it("returns each task's own result to its caller", async () => {
    const enqueue = createWriteQueue();
    const first = enqueue(async () => 1);
    const second = enqueue(async () => 2);
    expect(await first).toBe(1);
    expect(await second).toBe(2);
  });
});
