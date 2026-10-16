import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { createInstance } from "i18next";
import en from "../../i18n/locales/en.json";
import TransfersTab from "./TransfersTab";
import { invoke } from "@tauri-apps/api/core";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en } },
  interpolation: { escapeValue: false },
});
vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({ t: translations.t.bind(translations), i18n: translations }),
}));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => path),
  invoke: vi.fn(async () => ({})),
  isTauri: vi.fn(() => false),
}));

import { createGameState, createPlayer } from "./TransfersTab.testFixtures";

beforeEach(() => {
  vi.mocked(invoke).mockClear();
});

const renderMarket = () =>
  render(
    <TransfersTab
      gameState={createGameState([createPlayer({ team_id: "team-2", transfer_listed: true })])}
      onSelectPlayer={vi.fn()}
      onSelectTeam={vi.fn()}
      onGameUpdate={vi.fn()}
    />,
  );

/**
 * Given the Defenders chip is pressed
 * When its refinement panel opens
 * Then it is a labelled group of position toggles, not a dialog it does not behave as, and the
 *      chip says it is expanded.
 */
it("offers the position refinement as a labelled group, not a dialog", () => {
  renderMarket();
  const chip = screen.getByRole("button", { name: "Defenders" });

  fireEvent.click(chip);

  expect(screen.getByRole("group", { name: "Refine Defenders" })).toBeInTheDocument();
  expect(screen.queryByRole("dialog", { name: "Refine Defenders" })).not.toBeInTheDocument();
  expect(chip).toHaveAttribute("aria-expanded", "true");
});

/**
 * Given the refinement panel is open and focus is inside it
 * When Escape is pressed
 * Then the panel closes and focus returns to the chip that opened it.
 */
it("closes the refinement on Escape and returns focus to its chip", () => {
  renderMarket();
  const chip = screen.getByRole("button", { name: "Defenders" });
  chip.focus();
  fireEvent.click(chip);
  screen.getByRole("button", { name: "Center Back" }).focus();

  fireEvent.keyDown(screen.getByRole("button", { name: "Center Back" }), { key: "Escape" });

  expect(screen.queryByRole("group", { name: "Refine Defenders" })).not.toBeInTheDocument();
  expect(chip).toHaveFocus();
});
