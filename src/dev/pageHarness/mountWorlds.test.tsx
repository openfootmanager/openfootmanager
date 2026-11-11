import { act, within } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import { readStyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { mountWorldsHarness } from "./mountWorlds";

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

let mounted: Awaited<ReturnType<typeof mountWorldsHarness>> | null = null;

afterEach(async () => {
  await act(async () => mounted?.unmount());
  mounted = null;
  clearMocks();
  document.body.innerHTML = "";
});

async function mount(library: "populated" | "empty") {
  const root = document.createElement("div");
  document.body.append(root);
  await act(async () => {
    mounted = await mountWorldsHarness(root, library, readStyleGuideOptions(""), "en");
  });
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 300));
  });
  return root;
}

describe("the World Editor home on the page harness", () => {
  /**
   * Given the harness's populated library
   * When the home has settled
   * Then it lists the worlds (projects, one whose folder moved, and installed packages) and the
   *      fixture answered everything the home asked for.
   */
  it("lists a populated library and asks for nothing the fixture cannot answer", async () => {
    const root = await mount("populated");

    const view = within(root);
    expect(view.getByRole("heading", { level: 1, name: "Your worlds" })).toBeInTheDocument();
    expect(view.getAllByRole("row").length).toBeGreaterThan(5);
    expect(view.getByText("Nordic Leagues 2027", { selector: "span" })).toBeInTheDocument();
    expect(view.getAllByText("Can't be read").length).toBeGreaterThan(0);
    expect(window.__pageHarness?.unanswered).toEqual([]);
  });

  /**
   * Given the harness's empty library
   * When the home has settled
   * Then it says how to start instead of drawing an empty table.
   */
  it("says how to start when the library is empty", async () => {
    const root = await mount("empty");

    expect(within(root).getByText(/No worlds yet/)).toBeInTheDocument();
    expect(window.__pageHarness?.unanswered).toEqual([]);
  });
});
