import React from "react";
import ReactDOM from "react-dom/client";

import "../../App.css";
import { ThemeProvider } from "../../context/ThemeContext";
import { i18nReady } from "../../i18n";
import { StyleGuide } from "./StyleGuide";
import {
  applyStyleGuideOptions,
  readStyleGuideOptions,
  STYLE_GUIDE_THEME_KEY,
} from "./styleGuideOptions";

const options = readStyleGuideOptions(window.location.search);
applyStyleGuideOptions(options, document.documentElement, localStorage);

const rootElement = document.getElementById("root");
if (rootElement) {
  void i18nReady.then(() => {
    ReactDOM.createRoot(rootElement).render(
      <React.StrictMode>
        <ThemeProvider storageKey={STYLE_GUIDE_THEME_KEY}>
          <StyleGuide options={options} />
        </ThemeProvider>
      </React.StrictMode>,
    );
  });
}
