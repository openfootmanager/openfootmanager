import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../context/ThemeContext";
import { useSettingsStore } from "../store/settingsStore";
import Settings from "./Settings";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock, isTauri: () => false }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
  initReactI18next: { type: "3rdParty", init: () => {} },
}));

// The theme provider asks the system for its colour scheme; jsdom has no matchMedia.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

function renderSettings() {
  render(
    <MemoryRouter>
      <ThemeProvider>
        <Settings />
      </ThemeProvider>
    </MemoryRouter>,
  );
}

const savedSettings = () =>
  invokeMock.mock.calls
    .filter(([command]) => command === "save_settings")
    .map(([, args]) => (args as { settings: Record<string, unknown> }).settings);

const lastSaved = () => {
  const saved = savedSettings();
  return saved[saved.length - 1];
};

// The store as a fresh launch has it: a choice saved by one test must not be the next one's.
const freshStore = useSettingsStore.getState();

/** The backend answers with these saved settings; the rest are the store's defaults. */
function answerWithSaved(settings: Record<string, unknown>) {
  invokeMock.mockImplementation(async (command: string) => {
    if (command === "get_settings") {
      return {
        settings,
        currency: { code: "EUR", symbol: "€", exchange_rate: 1 },
        supported_currencies: [{ code: "EUR", symbol: "€", exchange_rate: 1 }],
      };
    }
    return command.startsWith("list_") ? [] : null;
  });
}

beforeEach(() => {
  useSettingsStore.setState(freshStore, true);
  invokeMock.mockReset();
  answerWithSaved({});
});

describe("Settings: the sidebar", () => {
  /**
   * Given the Display settings
   * When the player picks the Compact width, then Expand on click
   * Then each is saved, so the sidebar opens that way from now on.
   */
  it("saves the sidebar's width and behaviour", async () => {
    renderSettings();

    const width = await screen.findByRole("radiogroup", { name: "settings.sidebarWidth" });
    fireEvent.click(
      within(width).getByRole("radio", { name: "settings.sidebarWidthOptions.compact" }),
    );
    await waitFor(() => expect(lastSaved()).toMatchObject({ sidebar_width: "compact" }));

    const behaviour = screen.getByRole("radiogroup", { name: "settings.sidebarBehaviour" });
    fireEvent.click(
      within(behaviour).getByRole("radio", { name: "settings.sidebarBehaviourOptions.click" }),
    );
    await waitFor(() =>
      expect(lastSaved()).toMatchObject({
        sidebar_width: "compact",
        sidebar_behaviour: "click",
      }),
    );
  });

  /** Given the Display settings, then both rows offer every choice, named for the player. */
  it("offers every width and behaviour", async () => {
    renderSettings();
    const width = await screen.findByRole("radiogroup", { name: "settings.sidebarWidth" });
    for (const option of ["compact", "standard", "wide"]) {
      expect(
        within(width).getByRole("radio", { name: `settings.sidebarWidthOptions.${option}` }),
      ).toBeInTheDocument();
    }
    const behaviour = screen.getByRole("radiogroup", { name: "settings.sidebarBehaviour" });
    for (const option of ["normal", "hover", "click", "icons"]) {
      expect(
        within(behaviour).getByRole("radio", {
          name: `settings.sidebarBehaviourOptions.${option}`,
        }),
      ).toBeInTheDocument();
    }
  });
});

describe("Settings: the choices", () => {
  /**
   * Given the Settings page
   * When a screen reader reaches each row with a choice of a few
   * Then it is a radio group named by the row's own label, with the saved choice checked and the
   * only one in the tab order: theme, UI scale, sidebar width, sidebar behaviour, match speed.
   */
  it.each([
    ["settings.theme", "settings.themeOptions.dark"],
    ["settings.uiScale", "settings.uiScaleOptions.normal"],
    ["settings.sidebarWidth", "settings.sidebarWidthOptions.standard"],
    ["settings.sidebarBehaviour", "settings.sidebarBehaviourOptions.normal"],
    ["settings.matchSpeed", "settings.speeds.normal"],
  ])("draws %s as a radio group named by its row", async (row, saved) => {
    renderSettings();
    const group = await screen.findByRole("radiogroup", { name: row });
    const checked = within(group).getByRole("radio", { checked: true });
    expect(checked).toHaveAccessibleName(saved);
    expect(checked).toHaveAttribute("tabindex", "0");
    for (const other of within(group).getAllByRole("radio", { checked: false })) {
      expect(other).toHaveAttribute("tabindex", "-1");
    }
  });

  /**
   * Given the sidebar behaviour on Normal, with the focus on it
   * When the player presses the right arrow
   * Then the choice and the focus move to Expand on hover, and it is saved.
   */
  it("moves a choice with the arrow keys", async () => {
    renderSettings();
    const behaviour = await screen.findByRole("radiogroup", { name: "settings.sidebarBehaviour" });
    const normal = within(behaviour).getByRole("radio", {
      name: "settings.sidebarBehaviourOptions.normal",
    });
    normal.focus();
    fireEvent.keyDown(normal, { key: "ArrowRight" });
    await waitFor(() => expect(lastSaved()).toMatchObject({ sidebar_behaviour: "hover" }));
    expect(
      within(behaviour).getByRole("radio", { name: "settings.sidebarBehaviourOptions.hover" }),
    ).toHaveFocus();
  });

  /**
   * Given each on/off setting saved as the opposite of its default
   * When a screen reader reaches it on the Settings page
   * Then it is a switch named by its row's label, saying the saved state (not the default);
   * flipping it saves the other state.
   */
  it.each([
    ["settings.highContrast", "high_contrast", false],
    ["settings.matchCommentary", "show_match_commentary", true],
    ["settings.confirmAdvance", "confirm_advance", false],
    ["settings.continueToNextEvent", "continue_to_next_event", false],
    ["settings.skipShootoutPicker", "skip_shootout_picker", false],
    ["settings.autoSave", "auto_save", true],
  ] as const)("draws %s as a switch named by its row", async (row, field, byDefault) => {
    const saved = !byDefault;
    answerWithSaved({ [field]: saved });
    renderSettings();
    const control = await screen.findByRole("switch", { name: row });
    await waitFor(() => expect(control).toHaveAttribute("aria-checked", String(saved)));

    fireEvent.click(control);
    await waitFor(() => expect(lastSaved()).toMatchObject({ [field]: byDefault }));
  });
});
