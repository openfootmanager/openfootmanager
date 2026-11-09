// A driver that is not ours must never be mistaken for ours: see driver.mjs for why.
import assert from "node:assert/strict";
import net from "node:net";
import { test } from "node:test";

import { freePort, waitForOwnDriver } from "./driver.mjs";

/** A TCP server standing in for whatever is listening on a port. */
function listenOn(port) {
  return new Promise((resolve) => {
    const server = net.createServer((socket) => socket.destroy());
    server.listen(port, "127.0.0.1", () => resolve(server));
  });
}

const close = (server) => new Promise((resolve) => server.close(resolve));

/** The part of a ChildProcess the check reads. */
function childThat(state) {
  return state;
}

test("a free port can be bound straight after", async () => {
  const port = await freePort();
  const server = await listenOn(port);
  await close(server);
});

test("a driver that is alive and listening is ours", async () => {
  const port = await freePort();
  const server = await listenOn(port);
  const child = childThat({ exitCode: null, signalCode: null });
  await waitForOwnDriver(child, port, { graceMs: 50 });
  await close(server);
});

test("a stranger on the port is not mistaken for the driver we started", async () => {
  // Something else answers on the port, while the driver we spawned lost the race, failed to
  // bind, and exited. Connecting succeeds; the suite must still refuse.
  const port = await freePort();
  const stranger = await listenOn(port);
  const child = childThat({ exitCode: null, signalCode: null });
  setTimeout(() => {
    child.exitCode = 1;
  }, 100);
  await assert.rejects(
    waitForOwnDriver(child, port, { graceMs: 400 }),
    /something else owns that port|another process may own that port/,
  );
  await close(stranger);
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
