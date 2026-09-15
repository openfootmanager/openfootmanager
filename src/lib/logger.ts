import { error, info, warn } from "@tauri-apps/plugin-log";

/**
 * Writes frontend messages into the same log file the Rust side uses.
 *
 * Until this existed, nothing the frontend knew ever reached disk: the log plugin was configured
 * on the Rust side only, so the log folder a bug reporter zips up described the backend and was
 * silent about the render that actually broke. Every `console.error` in the app went to a devtools
 * console nobody had open.
 *
 * **Nothing here is allowed to throw or reject.** The plugin's functions are `invoke()` calls, so
 * they reject whenever the plugin is not there — outside Tauri, or if the capability is missing. A
 * rejection raised from inside the `unhandledrejection` handler in `src/main.tsx` would be a
 * rejection raised while handling a rejection, and on Linux/WebKitGTK an unhandled rejection
 * restarts the web process. The logger failing must never be worse than the bug it is reporting.
 */

type Sink = (message: string) => Promise<void> | void;

type ConsoleMethod = (...args: unknown[]) => void;

/**
 * Where a failed log write goes instead.
 *
 * While console forwarding is installed this holds the *pristine* console methods captured at
 * install time, never the patched ones. That is what makes the recursion structurally impossible
 * rather than merely unlikely: a plugin rejection lands on the original console, which does not
 * forward, so it cannot provoke another plugin call. A re-entrancy flag would not do the job — the
 * rejection arrives in a later microtask, long after any flag set around the call had been reset.
 */
let fallbackConsole: { error: ConsoleMethod; warn: ConsoleMethod } | undefined;

function fallbackFor(level: "error" | "warn"): ConsoleMethod {
  if (fallbackConsole) return fallbackConsole[level];
  // Read through `console` rather than capturing at module load, so a test that spies on it — and
  // a browser extension that wraps it — still sees the call.
  return level === "error"
    ? (...args: unknown[]) => console.error(...args)
    : (...args: unknown[]) => console.warn(...args);
}

function send(sink: Sink, message: string, level: "error" | "warn"): void {
  try {
    const result = sink(message);
    if (result instanceof Promise) {
      result.catch(() => fallbackFor(level)(message));
    }
  } catch {
    fallbackFor(level)(message);
  }
}

export function logError(message: string): void {
  send(error, message, "error");
}

export function logWarn(message: string): void {
  send(warn, message, "warn");
}

export function logInfo(message: string): void {
  send(info, message, "warn");
}

/** Renders `console.*` varargs into the single string the log file takes. */
export function formatLogArgs(args: unknown[]): string {
  return args
    .map((arg) => {
      if (typeof arg === "string") return arg;
      if (arg instanceof Error) return arg.stack ?? `${arg.name}: ${arg.message}`;
      try {
        return JSON.stringify(arg);
      } catch {
        // Cyclic objects, and anything with a throwing `toJSON`.
        return String(arg);
      }
    })
    .join(" ");
}

/**
 * Mirrors `console.error` and `console.warn` into the log file, and returns an undo function.
 *
 * The app already had well over a hundred `console.error` calls marking the places its authors
 * thought something could go wrong. Patching the two methods collects all of them at once, which
 * is a far better trade than editing every call site — and it keeps working for call sites added
 * later without anyone having to remember this module exists.
 *
 * `console.log`/`debug` are deliberately not forwarded: they are chatter, and a 5 MB log file
 * should be spent on things that went wrong.
 */
export function installConsoleForwarding(): () => void {
  // Kept unbound, so `restore()` can put back the exact function that was there. Binding here
  // would hand back a fresh wrapper each time, and repeated install/restore cycles would stack
  // one wrapper on another until the console was several layers deep.
  const originalError = console.error as ConsoleMethod;
  const originalWarn = console.warn as ConsoleMethod;
  const callError: ConsoleMethod = (...args) => originalError.call(console, ...args);
  const callWarn: ConsoleMethod = (...args) => originalWarn.call(console, ...args);
  const patchedError: ConsoleMethod = (...args) => {
    callError(...args);
    send(error, formatLogArgs(args), "error");
  };
  const patchedWarn: ConsoleMethod = (...args) => {
    callWarn(...args);
    send(warn, formatLogArgs(args), "warn");
  };

  fallbackConsole = { error: callError, warn: callWarn };
  console.error = patchedError;
  console.warn = patchedWarn;

  return () => {
    // Only stand down if nothing else has patched the console since; otherwise restoring would
    // silently remove theirs.
    if (console.error === patchedError) console.error = originalError;
    if (console.warn === patchedWarn) console.warn = originalWarn;
    fallbackConsole = undefined;
  };
}
