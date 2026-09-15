import React from "react";
import ReactDOM from "react-dom/client";
import { ThemeProvider } from "./context/ThemeContext";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { i18nReady } from "./i18n";
import { installConsoleForwarding, logError } from "./lib/logger";
import App from "./App";

// First, before anything else can fail: from here on `console.error` and `console.warn` also land
// in the log file a bug report carries. The app already had well over a hundred such calls
// marking the places it expected trouble, and until now every one of them wrote to a devtools
// console that nobody on a player's machine has open.
installConsoleForwarding();

// On Linux/WebKitGTK an unhandled promise rejection restarts the webview
// process. Swallow any that escape their own try-catch so the app stays up.
window.addEventListener("unhandledrejection", (event) => {
  event.preventDefault();
  console.error("Unhandled promise rejection:", event.reason);
});

// An uncaught synchronous error is printed by the browser itself rather than through
// `console.error`, so the forwarding above never sees it. This is the one that needs its own
// listener — and the one most likely to be the reason a player is filing a report.
window.addEventListener("error", (event) => {
  logError(
    `[window] Uncaught error: ${event.message} (${event.filename}:${event.lineno}:${event.colno})\n${
      event.error instanceof Error ? (event.error.stack ?? "") : ""
    }`,
  );
});

const rootElement = document.getElementById("root") as HTMLElement | null;

if (!rootElement) {
  throw new Error("Missing root element");
}

const root = ReactDOM.createRoot(rootElement);

function renderApp() {
  root.render(
    <React.StrictMode>
      <ThemeProvider>
        {/* Outside `App`, not inside its router: a throw in App's own effects or in the route
            resolution would otherwise escape past a boundary nested deeper and blank the window.
            ThemeProvider stays outermost so the fallback is rendered in the player's theme. */}
        <ErrorBoundary>
          <App />
        </ErrorBoundary>
      </ThemeProvider>
    </React.StrictMode>,
  );
}

void i18nReady
  .catch((error) => {
    console.error("Failed to initialize i18n:", error);
  })
  .finally(renderApp);

// Development tooling: runs the rendering benchmark when asked. `?ofmbench=<label>` is the handle
// for a browser; `VITE_OFM_BENCH=<label>` is the handle for the Tauri window, whose URL we do not
// control. See src/dev/benchUi.ts and docs/LINUX_GRAPHICS.md.
//
// The `import.meta.env.DEV` guard is load-bearing, not belt-and-braces: it is statically false in
// a production build, so the whole branch — and the dynamic import with it — is removed from the
// bundle. Without it the benchmark ships, and a release built in an environment that happens to
// export VITE_OFM_BENCH would hijack every launch with 11 seconds of scripted scrolling under a
// full-screen overlay.
if (import.meta.env.DEV) {
  const benchLabel =
    new URLSearchParams(window.location.search).get("ofmbench") || import.meta.env.VITE_OFM_BENCH;

  if (benchLabel) {
    void import("./dev/benchUi")
      .then(({ autoRunBench }) => autoRunBench(benchLabel))
      .catch((error) => console.error("[ofm-bench] failed:", error));
  }
}
