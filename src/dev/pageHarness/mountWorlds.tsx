import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import React from "react";
import ReactDOM from "react-dom/client";
import { MemoryRouter } from "react-router-dom";

import { WorldEditorHome } from "../../components/worldEditor/WorldEditorHome";
import { ThemeProvider } from "../../context/ThemeContext";
import { changeAppLanguage, type SupportedLanguageCode } from "../../i18n";
import { STYLE_GUIDE_THEME_KEY } from "../styleGuide/styleGuideOptions";
import type { StyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { createHarnessIpc } from "./harnessIpc";
import type { HarnessWorldLibrary } from "./harnessPage";
import { elementsPastEdge } from "./layoutProbe";
import { worldsFixture } from "./worldsFixture";

/**
 * Mounts the World Editor's home on a fixture library with the backend mocked: the one command it
 * asks (`get_worlds_library`) is answered, and any other is recorded as unanswered. The home's
 * callbacks do nothing here: this is for looking at the screen, not for driving the editor.
 */
export async function mountWorldsHarness(
  root: HTMLElement,
  library: HarnessWorldLibrary,
  _options: StyleGuideOptions,
  language: SupportedLanguageCode,
): Promise<ReactDOM.Root> {
  await changeAppLanguage(language);
  const { recentProjects, answer } = worldsFixture(library);
  const ipc = createHarnessIpc({ get_worlds_library: () => answer });
  mockWindows("main");
  mockIPC(ipc.handle, { shouldMockEvents: true });
  window.__pageHarness = { unanswered: ipc.unanswered, pastEdge: () => elementsPastEdge(root) };

  const reactRoot = ReactDOM.createRoot(root);
  const nothing = () => {};
  reactRoot.render(
    <React.StrictMode>
      <ThemeProvider storageKey={STYLE_GUIDE_THEME_KEY}>
        <MemoryRouter>
          <WorldEditorHome
            isBusy={false}
            errorMsg={null}
            recentProjects={recentProjects}
            onNewPackage={nothing}
            onOpenPackageFile={nothing}
            onOpenPackageFolder={nothing}
            onOpenRecent={nothing}
            onOpenInstalled={nothing}
          />
        </MemoryRouter>
      </ThemeProvider>
    </React.StrictMode>,
  );
  return reactRoot;
}
