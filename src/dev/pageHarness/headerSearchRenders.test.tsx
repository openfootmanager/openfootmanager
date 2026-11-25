import { act, fireEvent, within } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import { useGameStore } from "../../store/gameStore";
import { readStyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { mountPageHarness } from "./mount";

/** How many times the Tactics tab has rendered, counted by the stand-in below. */
const tacticsRenders = vi.hoisted(() => ({ count: 0 }));

// The tab itself is heavy and not what is under test: a stand-in that counts its renders, mounted by
// the real Dashboard exactly where the real tab is.
vi.mock("../../components/tactics/TacticsTab", () => ({
  default: () => {
    tacticsRenders.count += 1;
    return <div data-testid="tactics-stand-in" />;
  },
}));

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
  await act(async () => mounted?.unmount());
  mounted = null;
  clearMocks();
  useGameStore.getState().clearGame();
  document.body.innerHTML = "";
});

describe("the dashboard header's search", () => {
  /**
   * Given the dashboard on the Tactics tab, settled
   * When a name is typed into the header's search a character at a time
   * Then the box shows what was typed, and the Tactics tab did not render again: a keystroke
   *      re-renders the search and its results, not the dashboard and the tab under it (on a weak
   *      PC each keystroke of a whole-dashboard render took most of a second).
   */
  it("does not render the Tactics tab again on a keystroke", async () => {
    const root = document.createElement("div");
    document.body.append(root);
    await act(async () => {
      mounted = await mountPageHarness(root, "tactics", readStyleGuideOptions(""), "en");
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, SETTLE_MS));
    });
    const search = within(root).getByRole("textbox", { name: "Search" }) as HTMLInputElement;
    const rendersBefore = tacticsRenders.count;
    expect(rendersBefore).toBeGreaterThan(0);

    for (const typed of ["a", "an", "ang", "ange"]) {
      await act(async () => {
        fireEvent.change(search, { target: { value: typed } });
      });
      expect(search.value).toBe(typed);
    }

    expect(tacticsRenders.count).toBe(rendersBefore);
  }, 15_000);
});
