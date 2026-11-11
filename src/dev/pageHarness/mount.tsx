import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import React from "react";
import ReactDOM from "react-dom/client";

import { ThemeProvider } from "../../context/ThemeContext";
import { changeAppLanguage, type SupportedLanguageCode } from "../../i18n";
import { STYLE_GUIDE_THEME_KEY, type StyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { fixtureHandlers } from "./fixtureAnswers";
import { gameState } from "./fixture.generated";
import { createHarnessIpc } from "./harnessIpc";
import { elementsPastEdge } from "./layoutProbe";
import type { HarnessPage, HarnessProfile } from "./harnessPage";
import { PageHarness } from "./PageHarness";

declare global {
  interface Window {
    /** What a screenshot or layout script reads once the page has settled. */
    __pageHarness?: { unanswered: string[]; pastEdge: () => string[] };
  }
}

/**
 * Mounts `page` on the fixture career with the backend mocked: invoke, events and the window.
 * Returns the React root, so a test can unmount the page before it clears the mocks.
 */
export async function mountPageHarness(
  root: HTMLElement,
  page: HarnessPage,
  options: StyleGuideOptions,
  language: SupportedLanguageCode,
  profile: HarnessProfile = "own",
): Promise<ReactDOM.Root> {
  await changeAppLanguage(language);
  const ipc = createHarnessIpc(
    fixtureHandlers(options, language, page === "profile" ? profile : null),
  );
  mockWindows("main");
  mockIPC(ipc.handle, { shouldMockEvents: true });
  window.__pageHarness = { unanswered: ipc.unanswered, pastEdge: () => elementsPastEdge(root) };

  const manager = gameState.manager;
  const reactRoot = ReactDOM.createRoot(root);
  reactRoot.render(
    <React.StrictMode>
      <ThemeProvider storageKey={STYLE_GUIDE_THEME_KEY}>
        <PageHarness
          page={page}
          profile={profile}
          managerName={`${manager.first_name} ${manager.last_name}`}
        />
      </ThemeProvider>
    </React.StrictMode>,
  );
  return reactRoot;
}
