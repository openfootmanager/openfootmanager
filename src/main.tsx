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
