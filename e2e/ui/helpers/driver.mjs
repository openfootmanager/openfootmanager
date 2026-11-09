// The WebDriver server the suite talks to has to be the one the suite started.
//
// tauri-driver launches the app named in the session's capabilities, with the environment it was
// started in. A driver that was already running (a developer's, left idle on the default port) has
// the developer's environment: the suite would connect to it and the app would open on the real
// display, with the real save directory. So the suite starts its own driver on ports it picked.
import net from "node:net";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** A TCP port nothing is listening on right now. */
export function freePort() {
  return new Promise((resolve, reject) => {
    const probe = net.createServer();
    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const { port } = probe.address();
      probe.close(() => resolve(port));
    });
  });
}

function accepts(port) {
  return new Promise((resolve) => {
    const socket = net.connect({ port, host: "127.0.0.1" });
    socket.once("connect", () => {
      socket.destroy();
      resolve(true);
    });
    socket.once("error", () => resolve(false));
  });
}

const hasExited = (child) => child.exitCode !== null || child.signalCode !== null;

/** Resolves once something is listening on `port`; says "exited" if the child is already gone. */
export async function waitForOwnDriver(child, port, { timeoutMs = 30_000, intervalMs = 100 } = {}) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (hasExited(child)) {
      throw new Error(`tauri-driver exited (${child.exitCode ?? child.signalCode}) before listening on ${port}.`);
    }
    if (await accepts(port)) return;
    await sleep(intervalMs);
  }
  throw new Error(`tauri-driver was not listening on ${port} after ${timeoutMs} ms.`);
}
