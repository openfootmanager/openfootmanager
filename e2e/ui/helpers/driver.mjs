// The WebDriver server the suite talks to has to be the one the suite started.
//
// tauri-driver launches the app named in the session's capabilities, with the environment it was
// started in. A driver that was already running (a developer's, left idle on the default port) has
// the developer's environment: the suite would connect to it and the app would open on the real
// display, with the real save directory. So the suite starts its own driver on ports it picked,
// and does not go on until that very process owns the socket listening on its port.
import { readdirSync, readFileSync, readlinkSync } from "node:fs";
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

/** What the ownership check reads of /proc; tests pass a stand-in. */
const realProc = {
  readFile: (file) => readFileSync(file, "utf8"),
  listFds(pid) {
    const directory = `/proc/${pid}/fd`;
    const targets = [];
    for (const entry of readdirSync(directory)) {
      try {
        targets.push(readlinkSync(`${directory}/${entry}`));
      } catch {
        // The descriptor was closed between the listing and the read.
      }
    }
    return targets;
  },
};

/** The inodes of the sockets listening on `port`, from the kernel's IPv4 and IPv6 tables. */
export function listeningInodes(port, procfs = realProc) {
  const inodes = new Set();
  for (const file of ["/proc/net/tcp", "/proc/net/tcp6"]) {
    let table;
    try {
      table = procfs.readFile(file);
    } catch {
      continue;
    }
    for (const line of table.split("\n").slice(1)) {
      const columns = line.trim().split(/\s+/);
      if (columns.length < 10) continue;
      const [, local, , state] = columns;
      // State 0A is LISTEN; the local address is "HEXADDRESS:HEXPORT".
      if (state === "0A" && Number.parseInt(local.split(":")[1], 16) === port) {
        inodes.add(columns[9]);
      }
    }
  }
  return inodes;
}

/** The inodes of the sockets the process holds open. */
function socketInodesOf(pid, procfs) {
  const inodes = new Set();
  let targets;
  try {
    targets = procfs.listFds(pid);
  } catch {
    return inodes;
  }
  for (const target of targets) {
    const match = /^socket:\[(\d+)\]$/.exec(target);
    if (match) inodes.add(match[1]);
  }
  return inodes;
}

/** Whether a socket listening on `port` is held open by `child`. */
export function childOwnsListener(child, port, procfs = realProc) {
  const mine = socketInodesOf(child.pid, procfs);
  for (const inode of listeningInodes(port, procfs)) {
    if (mine.has(inode)) return true;
  }
  return false;
}

/**
 * Resolves once `child`, the driver this process started, owns the socket listening on `port`.
 *
 * Something answering on the port is not enough, and neither is the child staying alive: if
 * another driver took the port, the one we started either exits (refused at once) or lingers
 * without it. So the listening socket itself must be one the child holds open, read from /proc.
 * A child that is slow to bind is given until the timeout.
 */
export async function waitForOwnDriver(
  child,
  port,
  { timeoutMs = 30_000, intervalMs = 100, graceMs = 750, procfs = realProc } = {},
) {
  const deadline = Date.now() + timeoutMs;
  let answered = false;
  let waitedOutGrace = false;
  while (Date.now() < deadline) {
    if (hasExited(child)) {
      throw new Error(
        `tauri-driver exited (${child.exitCode ?? child.signalCode}) before it owned port ${port}; ` +
          "something else owns that port, or the driver could not start.",
      );
    }
    if (await accepts(port)) {
      answered = true;
      if (!waitedOutGrace) {
        // Give a driver that lost the race for the port time to fail and exit.
        await sleep(graceMs);
        waitedOutGrace = true;
        continue;
      }
      if (childOwnsListener(child, port, procfs)) return;
    }
    await sleep(intervalMs);
  }
  throw new Error(
    answered
      ? `port ${port} answers, but not the driver we started: something else holds its listening socket.`
      : `tauri-driver was not listening on ${port} after ${timeoutMs} ms.`,
  );
}
