import React from "react";
import ReactDOM from "react-dom/client";

import "../../App.css";
import { ThemeProvider } from "../../context/ThemeContext";
import { i18nReady } from "../../i18n";
import { StyleGuide } from "./StyleGuide";
import { readStyleGuideOptions } from "./styleGuideOptions";

const options = readStyleGuideOptions(window.location.search);
const root = document.documentElement;
root.classList.toggle("dark", options.theme === "dark");
root.classList.toggle("density-compact", options.density === "compact");
root.classList.toggle("high-contrast", options.highContrast);
root.style.fontSize = options.rootFontSize;
// ThemeProvider reads its starting theme from here, so the toggle in the guide agrees with the URL.
localStorage.setItem("ofm-theme", options.theme);

const rootElement = document.getElementById("root");
if (rootElement) {
  void i18nReady.then(() => {
    ReactDOM.createRoot(rootElement).render(
      <React.StrictMode>
        <ThemeProvider>
          <StyleGuide options={options} />
        </ThemeProvider>
      </React.StrictMode>,
    );
  });
}
