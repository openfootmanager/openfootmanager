import { act } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import { useGameStore } from "../../store/gameStore";
import { readStyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { HARNESS_PAGES, type HarnessPage } from "./harnessPage";
import { mountPageHarness } from "./mount";

/** Long enough for the dashboard and the page to ask for everything they ask for on mount. */
const SETTLE_MS = 500;

beforeAll(async () => {
  // jsdom has no matchMedia; ThemeProvider asks it for the system theme.
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  });
  await i18nReady;
});

let mounted: Awaited<ReturnType<typeof mountPageHarness>> | null = null;

afterEach(async () => {
  // Unmounted before the mocks go: the dashboard's event listeners unregister through them.
  await act(async () => mounted?.unmount());
  mounted = null;
  clearMocks();
  useGameStore.getState().clearGame();
  document.body.innerHTML = "";
});

describe("the page harness's pages", () => {
  /**
   * Given each page the harness can mount, on the fixture career
   * When it has settled
   * Then the page rendered (it did not fall back to the "left the dashboard" alert) and the
   *      fixture answered every command it asked for. A command a page starts asking for with
   *      no answer in fixtureAnswers.ts fails here, in the required frontend job, rather than only
   *      in the layout check, which is not a required job and so can sit red unnoticed.
   *      A command a page asks for later than the settle time is not seen: this guard can miss a
   *      late command, never fail on a good page.
   */
  it.each(Object.keys(HARNESS_PAGES) as HarnessPage[])(
    "asks for no command the fixture cannot answer: %s",
    async (page) => {
      const root = document.createElement("div");
      document.body.append(root);

      // The first render is flushed inside act, so it has committed before the settle timer starts.
      await act(async () => {
        mounted = await mountPageHarness(root, page, readStyleGuideOptions(""), "en");
      });
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, SETTLE_MS));
      });

      // An empty unanswered list means nothing only if the page rendered: a harness that never
      // activated the game draws nothing and asks for nothing.
      expect(useGameStore.getState().hasActiveGame).toBe(true);
      expect(root.querySelectorAll("*").length).toBeGreaterThan(50);
      expect(root.querySelector('[role="alert"]')).toBeNull();
      expect(window.__pageHarness?.unanswered).toEqual([]);
    },
    15_000,
  );
});
