// A driver that is not ours must never be mistaken for ours: see driver.mjs for why.
//
// `driver` is imported as a namespace so that a test of a function that does not exist yet fails
// on its own, instead of the whole file failing to load.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import net from "node:net";
import { after, test } from "node:test";

import * as driver from "./driver.mjs";

// Everything a test starts is stopped when the file is done, whether or not its assertions held.
// A listener or a child left running keeps the process alive: a failing test would then hang the
// whole run instead of reporting.
const servers = [];
const children = [];
after(() => {
  for (const server of servers) server.close();
  for (const child of children) child.kill();
});

/** A TCP server in this process, standing in for whatever else is listening on a port. */
function listenOn(port) {
  return new Promise((resolve) => {
    const server = net.createServer((socket) => socket.destroy());
    servers.push(server);
    server.listen(port, "127.0.0.1", () => resolve(server));
  });
}

/** A real child process that listens on `port`, once it says so. */
function spawnListener(port) {
  return new Promise((resolve, reject) => {
    const script = `require("net").createServer().listen(${port}, "127.0.0.1", () => console.log("ready"))`;
    const child = spawn(process.execPath, ["-e", script], { stdio: ["ignore", "pipe", "inherit"] });
    children.push(child);
    child.once("error", reject);
    child.stdout.once("data", () => resolve(child));
  });
}

/** A real child process that stays alive and listens on nothing. */
function spawnIdle() {
  const child = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], {
    stdio: "ignore",
  });
  children.push(child);
  return child;
}

/** The part of a ChildProcess the check reads, for a child that is never really started. */
const pretendChild = (pid) => ({ pid, exitCode: null, signalCode: null });

const hex = (port) => port.toString(16).toUpperCase().padStart(4, "0");

/** A line of /proc/net/tcp for a socket on 127.0.0.1:`port`. */
const tcpLine = (port, inode, state = "0A") =>
  `   0: 0100007F:${hex(port)} 00000000:0000 ${state} 00000000:00000000 00:00000000 00000000  1000        0 ${inode} 1 0000000000000000 100 0 0 10 0`;

const TCP_HEADER =
  "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode";

/** A stand-in for /proc: the two socket tables, and the descriptors each pid holds. */
function pretendProc({ tcp = [], tcp6 = [], fds = {} }) {
  const table = (lines) => [TCP_HEADER, ...lines].join("\n");
  return {
    readFile(file) {
      if (file === "/proc/net/tcp") return table(tcp);
      if (file === "/proc/net/tcp6") return table(tcp6);
      throw new Error(`no such file ${file}`);
    },
    listFds: (pid) => fds[pid] ?? [],
  };
}

test("a free port can be bound straight after", async () => {
  const port = await driver.freePort();
  await listenOn(port);
});

test("the sockets listening on a port are read from both socket tables", () => {
  const proc = pretendProc({
    tcp: [tcpLine(8080, "111"), tcpLine(8080, "222", "01"), tcpLine(9090, "333")],
    tcp6: [tcpLine(8080, "444")],
  });
  // 111 and 444 listen on 8080; 222 is an established connection and 333 is another port.
  assert.deepEqual([...driver.listeningInodes(8080, proc)].sort(), ["111", "444"]);
});

test("a driver that holds the listening socket is ours", async () => {
  const port = await driver.freePort();
  await listenOn(port);
  const proc = pretendProc({ tcp: [tcpLine(port, "111")], fds: { 4321: ["socket:[111]"] } });
  await driver.waitForOwnDriver(pretendChild(4321), port, { graceMs: 50, procfs: proc });
});

test("a stranger answering beside the live driver we started is refused", async () => {
  // The port is answered, and the child we spawned is alive past the grace period: but the socket
  // listening on that port is somebody else's, so the session would go to the wrong driver.
  const port = await driver.freePort();
  await listenOn(port);
  const proc = pretendProc({
    tcp: [tcpLine(port, "111")],
    fds: { 4321: ["socket:[222]", "pipe:[333]"] },
  });
  await assert.rejects(
    driver.waitForOwnDriver(pretendChild(4321), port, {
      graceMs: 50,
      timeoutMs: 600,
      intervalMs: 50,
      procfs: proc,
    }),
    /not the driver we started/,
  );
});

test("a real child that listens on the port is ours (real /proc)", async () => {
  const port = await driver.freePort();
  const child = await spawnListener(port);
  await driver.waitForOwnDriver(child, port, { graceMs: 50 });
});

test("a stranger answering beside a live real child is refused (real /proc)", async () => {
  const port = await driver.freePort();
  await listenOn(port);
  const child = spawnIdle();
  await assert.rejects(
    driver.waitForOwnDriver(child, port, { graceMs: 50, timeoutMs: 1500, intervalMs: 50 }),
    /not the driver we started/,
  );
});

test("a stranger on the port is refused when the driver we started has exited", async () => {
  // The driver lost the race for the port, failed to bind, and exited; the stranger still answers.
  const port = await driver.freePort();
  await listenOn(port);
  const child = pretendChild(4321);
  setTimeout(() => {
    child.exitCode = 1;
  }, 100);
  await assert.rejects(
    driver.waitForOwnDriver(child, port, { graceMs: 400 }),
    /something else owns that port|another process may own that port/,
  );
});

test("a driver that exits before listening is refused at once", async () => {
  const port = await driver.freePort();
  const child = { pid: 4321, exitCode: 1, signalCode: null };
  await assert.rejects(driver.waitForOwnDriver(child, port), /exited \(1\)/);
});

test("a driver that never listens times out and says so", async () => {
  const port = await driver.freePort();
  await assert.rejects(
    driver.waitForOwnDriver(pretendChild(4321), port, { timeoutMs: 300, intervalMs: 50 }),
    /not listening/,
  );
});
