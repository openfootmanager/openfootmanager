import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../context/ThemeContext";
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

describe("Settings: the sidebar", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_settings") {
        return {
          settings: {},
          currency: { code: "EUR", symbol: "€", exchange_rate: 1 },
          supported_currencies: [{ code: "EUR", symbol: "€", exchange_rate: 1 }],
        };
      }
      return command.startsWith("list_") ? [] : null;
    });
  });

  /**
   * Given the Display settings
   * When the player picks the Compact width, then Expand on click
   * Then each is saved, so the sidebar opens that way from now on.
   */
  it("saves the sidebar's width and behaviour", async () => {
    renderSettings();

    fireEvent.click(
      await screen.findByRole("button", { name: "settings.sidebarWidthOptions.compact" }),
    );
    await waitFor(() => expect(lastSaved()).toMatchObject({ sidebar_width: "compact" }));

    fireEvent.click(screen.getByRole("button", { name: "settings.sidebarBehaviourOptions.click" }));
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
    expect(await screen.findByText("settings.sidebarWidth")).toBeInTheDocument();
    expect(screen.getByText("settings.sidebarBehaviour")).toBeInTheDocument();
    for (const option of ["compact", "standard", "wide"]) {
      expect(
        screen.getByRole("button", { name: `settings.sidebarWidthOptions.${option}` }),
      ).toBeInTheDocument();
    }
    for (const option of ["normal", "hover", "click", "icons"]) {
      expect(
        screen.getByRole("button", { name: `settings.sidebarBehaviourOptions.${option}` }),
      ).toBeInTheDocument();
    }
  });
});
