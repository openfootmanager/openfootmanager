import React from "react";
import ReactDOM from "react-dom/client";
import { ThemeProvider } from "./context/ThemeContext";
import { i18nReady } from "./i18n";
import App from "./App";

// On Linux/WebKitGTK an unhandled promise rejection restarts the webview
// process. Swallow any that escape their own try-catch so the app stays up.
window.addEventListener("unhandledrejection", (event) => {
  event.preventDefault();
  console.error("Unhandled promise rejection:", event.reason);
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
        <App />
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
    new URLSearchParams(window.location.search).get("ofmbench") ||
    import.meta.env.VITE_OFM_BENCH;

  if (benchLabel) {
    void import("./dev/benchUi")
      .then(({ autoRunBench }) => autoRunBench(benchLabel))
      .catch((error) => console.error("[ofm-bench] failed:", error));
  }
}
