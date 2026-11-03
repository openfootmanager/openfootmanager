import React from "react";
import ReactDOM from "react-dom/client";

import "../../App.css";
import { ThemeProvider } from "../../context/ThemeContext";
import { i18nReady } from "../../i18n";
import { readHarnessLanguage, readHarnessPage } from "../pageHarness/harnessPage";
import { StyleGuide } from "./StyleGuide";
import {
  applyStyleGuideOptions,
  readStyleGuideOptions,
  STYLE_GUIDE_THEME_KEY,
} from "./styleGuideOptions";

const options = readStyleGuideOptions(window.location.search);
applyStyleGuideOptions(options, document.documentElement, localStorage);

const rootElement = document.getElementById("root");
const page = readHarnessPage(window.location.search);
if (rootElement && page) {
  // ?page= mounts a real page on the fixture career instead of the guide (src/dev/pageHarness).
  void i18nReady
    .then(() => import("../pageHarness/mount"))
    .then(({ mountPageHarness }) =>
      mountPageHarness(rootElement, page, options, readHarnessLanguage(window.location.search)),
    );
} else if (rootElement) {
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
