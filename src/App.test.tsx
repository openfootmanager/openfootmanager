import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import i18n, { i18nReady } from "./i18n";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => false,
  invoke: vi.fn(),
}));

vi.mock("./store/settingsStore", () => ({
  useSettingsStore: () => ({
    settings: { language: "en", ui_scale: 1, high_contrast: false },
    loaded: true,
    loadSettings: vi.fn(),
  }),
}));

beforeAll(async () => {
  await i18nReady;
  await i18n.changeLanguage("en");
});

describe("App", () => {
  /** Given a route that has not loaded yet, when the app renders, then a loading status shows. */
  it("announces a loading status while the route loads", () => {
    render(<App />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading...");
  });
});
