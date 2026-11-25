import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";

const logError = vi.fn();
vi.mock("./lib/logger", () => ({
  logError: (message: string) => logError(message),
}));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("./store/settingsStore", () => ({
  useSettingsStore: () => ({
    settings: { ui_scale: "normal", high_contrast: false, language: "en" },
    loaded: true,
    loadSettings: vi.fn(),
  }),
}));

vi.mock("./pages/MainMenu", () => ({
  default: () => {
    throw new Error("the page exploded");
  },
}));

describe("App error boundary", () => {
  beforeEach(() => {
    logError.mockReset();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /**
   * Given a page that throws while rendering
   * When the app renders it
   * Then the error screen stands where a blank window would have been, and the error is logged.
   */
  it("shows the error screen when a page throws", async () => {
    render(<App />);

    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(logError).toHaveBeenCalledWith(expect.stringContaining("the page exploded"));
  });
});
