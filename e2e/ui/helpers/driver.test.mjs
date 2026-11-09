// A driver that is not ours must never be mistaken for ours: see driver.mjs for why.
import assert from "node:assert/strict";
import net from "node:net";
import { after, test } from "node:test";

import { freePort, waitForOwnDriver } from "./driver.mjs";

// Every server a test opens is closed when the file is done, whether or not its assertions held.
// A listener left open keeps the process alive: a failing test would then hang the whole run
// instead of reporting.
const open = [];
after(() => {
  for (const server of open) server.close();
});

/** A TCP server standing in for whatever is listening on a port. */
function listenOn(port) {
  return new Promise((resolve) => {
    const server = net.createServer((socket) => socket.destroy());
    open.push(server);
    server.listen(port, "127.0.0.1", () => resolve(server));
  });
}

/** The part of a ChildProcess the check reads. */
function childThat(state) {
  return state;
}

test("a free port can be bound straight after", async () => {
  const port = await freePort();
  await listenOn(port);
});

test("a driver that is alive and listening is ours", async () => {
  const port = await freePort();
  await listenOn(port);
  const child = childThat({ exitCode: null, signalCode: null });
  await waitForOwnDriver(child, port, { graceMs: 50 });
});

test("a stranger on the port is not mistaken for the driver we started", async () => {
  // Something else answers on the port, while the driver we spawned lost the race, failed to
  // bind, and exited. Connecting succeeds; the suite must still refuse.
  const port = await freePort();
  await listenOn(port);
  const child = childThat({ exitCode: null, signalCode: null });
  setTimeout(() => {
    child.exitCode = 1;
  }, 100);
  await assert.rejects(
    waitForOwnDriver(child, port, { graceMs: 400 }),
    /something else owns that port|another process may own that port/,
  );
});

test("a driver that exits before listening is refused at once", async () => {
  const port = await freePort();
  const child = childThat({ exitCode: 1, signalCode: null });
  await assert.rejects(waitForOwnDriver(child, port), /exited \(1\)/);
});

test("a driver that never listens times out and says so", async () => {
  const port = await freePort();
  const child = childThat({ exitCode: null, signalCode: null });
  await assert.rejects(
    waitForOwnDriver(child, port, { timeoutMs: 300, intervalMs: 50 }),
    /not listening/,
  );
});
