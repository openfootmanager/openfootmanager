import type { GameStateData } from "../../store/gameStore";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { createInstance } from "i18next";
import en from "../../i18n/locales/en.json";
import de from "../../i18n/locales/de.json";
import TransfersTab from "./TransfersTab";
import { invoke } from "@tauri-apps/api/core";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en }, de: { translation: de } },
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

import { createPlayer, createGameState } from "./TransfersTab.testFixtures";

beforeEach(() => {
  vi.mocked(invoke).mockClear();
});

/** Given a workspace already open, when the parent receives accepted saved data, then history and action state use the current player rather than the opening snapshot. */
it.each(["transfer", "loan"] as const)(
  "refreshes an open workspace after a %s acceptance",
  (kind) => {
    const player = createPlayer({
      team_id: "team-2",
      transfer_listed: true,
      loan_listed: true,
      transfer_offers: [],
    });
    const state = createGameState([player]);
    const props = { onSelectPlayer: vi.fn(), onSelectTeam: vi.fn(), onGameUpdate: vi.fn() };
    const { rerender } = render(<TransfersTab gameState={state} {...props} />);
    const trigger = screen.getByRole("button", { name: "Make Offer" });
    trigger.focus();
    fireEvent.click(trigger);
    const updated = {
      ...player,
      transfer_offers:
        kind === "transfer"
          ? [
              {
                id: "agreed",
                from_team_id: "team-1",
                fee: 700001,
                wage_offered: 0,
                last_manager_fee: null,
                negotiation_round: 1,
                suggested_counter_fee: null,
                status: "PendingRegistration" as const,
                date: "2026-12-20",
                registration_date: "2027-01-02",
              },
            ]
          : [],
      loan_offers:
        kind === "loan"
          ? [
              {
                id: "agreed-loan",
                from_team_id: "team-1",
                parent_team_id: "team-2",
                start_date: "2027-01-02",
                end_date: "2027-06-30",
                wage_contribution_pct: 75,
                status: "PendingRegistration" as const,
                date: "2026-12-20",
              },
            ]
          : [],
    };
    rerender(<TransfersTab gameState={createGameState([updated])} {...props} />);
    const dialog = screen.getByRole("dialog", { name: player.full_name });
    expect(within(dialog).getByRole("region", { name: "Offer history" })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Make Transfer Bid" })).toBeDisabled();
    expect(within(dialog).getByRole("button", { name: "Make Loan Offer" })).toBeDisabled();
    fireEvent.click(within(dialog).getByRole("button", { name: "Back" }));
    expect(screen.getByRole("button", { name: "Players (0)" })).toHaveFocus();
  },
);

/** Given a reloaded deferred agreement, when Offers opens its history, then persisted details are reachable by keyboard without starting a bid. */
it.each(["transfer", "loan"] as const)(
  "opens saved %s agreement history from the offers view",
  (kind) => {
    const player = createPlayer({
      team_id: "team-2",
      transfer_listed: true,
      transfer_offers:
        kind === "transfer"
          ? [
              {
                id: "deferred",
                from_team_id: "team-1",
                fee: 700001,
                wage_offered: 0,
                last_manager_fee: null,
                negotiation_round: 1,
                suggested_counter_fee: null,
                status: "PendingRegistration",
                date: "2026-12-20",
                registration_date: "2027-01-02",
              },
            ]
          : [],
      loan_offers:
        kind === "loan"
          ? [
              {
                id: "deferred-loan",
                from_team_id: "team-1",
                parent_team_id: "team-2",
                start_date: "2027-01-02",
                end_date: "2027-06-30",
                wage_contribution_pct: 75,
                status: "PendingRegistration",
                date: "2026-12-20",
              },
            ]
          : [],
    });
    const restored: GameStateData = JSON.parse(JSON.stringify(createGameState([player])));
    render(<TransfersTab gameState={restored} onSelectPlayer={vi.fn()} onSelectTeam={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Offers (1)" }));
    fireEvent.click(screen.getByRole("button", { name: "View offer history" }));
    const dialog = screen.getByRole("dialog", { name: player.full_name });
    const history = within(dialog).getByRole("region", { name: "Offer history" });
    expect(within(history).getByText("Pending registration")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Make Transfer Bid" })).toBeDisabled();
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith(
      "preview_transfer_bid_financial_impact",
      expect.anything(),
    );
  },
);

/** Given a reloaded offer from a missing club, when the Offers table is read in German, then its club fallback is translated. */
it.each(["transfer", "loan"] as const)(
  "translates the %s offer club in the offers table",
  async (kind) => {
    await translations.changeLanguage("de");
    const player = createPlayer({
      transfer_offers:
        kind === "transfer"
          ? [
              {
                id: "missing-transfer",
                from_team_id: "missing-club",
                fee: 700001,
                wage_offered: 0,
                last_manager_fee: null,
                negotiation_round: 1,
                suggested_counter_fee: null,
                status: "Rejected",
                date: "2026-12-20",
              },
            ]
          : [],
      loan_offers:
        kind === "loan"
          ? [
              {
                id: "missing-loan",
                from_team_id: "missing-club",
                parent_team_id: "team-1",
                start_date: "2027-01-02",
                end_date: "2027-06-30",
                wage_contribution_pct: 75,
                status: "Rejected",
                date: "2026-12-20",
              },
            ]
          : [],
    });
    render(
      <TransfersTab
        gameState={createGameState([player])}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Angebote \(1\)/ }));
    expect(screen.getByText("Unbekannt")).toBeInTheDocument();
    expect(screen.queryByText("Unknown")).not.toBeInTheDocument();
  },
);

afterEach(async () => {
  await translations.changeLanguage("en");
});
