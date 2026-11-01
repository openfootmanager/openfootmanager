import { fireEvent, render, screen, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { createInstance } from "i18next";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import en from "../../i18n/locales/en.json";
import { formatExactMoney, formatWeeklyWage } from "../../lib/helpers";
import type { VisibleFigures } from "../../lib/playerKnowledge";
import { useSettingsStore } from "../../store/settingsStore";
import TransfersTab from "./TransfersTab";
import {
  createGameState,
  createPlayer,
  marketPageOf,
  playersOfScope,
} from "./TransfersTab.testFixtures";

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
  invoke: vi.fn(),
  isTauri: vi.fn(() => false),
}));

const REAL_WAGE = 12345;
const unscouted = createPlayer({
  id: "unscouted",
  full_name: "Never Scouted",
  team_id: "team-2",
  transfer_listed: true,
  wage: REAL_WAGE,
});
const ownListed = createPlayer({
  id: "own-listed",
  full_name: "Own Listed",
  team_id: "team-1",
  transfer_listed: true,
  wage: REAL_WAGE,
});
const offered = createPlayer({
  id: "offered",
  full_name: "Offered Player",
  team_id: "team-2",
  transfer_listed: true,
  transfer_offers: [
    {
      id: "bid-1",
      from_team_id: "team-1",
      fee: 700001,
      wage_offered: 0,
      last_manager_fee: null,
      negotiation_round: 1,
      suggested_counter_fee: null,
      status: "Pending",
      date: "2026-12-20",
      registration_date: null,
    },
  ],
});
const MARKET = [unscouted, ownListed];

// What the market page lets the club see of each: nothing of an unscouted rival, everything of its own.
const FIGURES: Record<string, VisibleFigures> = {
  unscouted: { ovr: null, wage: { kind: "hidden" } },
  "own-listed": { ovr: 72, wage: { kind: "exact", weekly: REAL_WAGE } },
  offered: { ovr: 70, wage: { kind: "hidden" } },
};

function withFigures(players: typeof MARKET) {
  const page = marketPageOf(players);
  return { ...page, items: page.items.map((item) => ({ ...item, ...FIGURES[item.id] })) };
}

function setListView(view: "table" | "cards") {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, list_view: view },
  });
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (command: string, payload?: unknown) =>
    command === "get_players_page"
      ? withFigures(
          playersOfScope(payload, {
            market: MARKET,
            user_listed: [],
            with_offers: [offered],
          }),
        )
      : {},
  );
  setListView("cards");
});
afterEach(() => setListView("table"));

const renderCentre = (onSelectPlayer = vi.fn()) =>
  render(
    <TransfersTab
      gameState={createGameState([...MARKET, offered])}
      onSelectPlayer={onSelectPlayer}
      onSelectTeam={vi.fn()}
      onGameUpdate={vi.fn()}
    />,
  );

/**
 * Given list_view is "cards" and a market of an unscouted rival and the club's own listed player
 * When the transfer centre lists them
 * Then each is a card, there is no table, the rival's card shows dashes for rating and wage and
 *      nothing of his real wage, and the club's own shows its rating and exact wage.
 */
it("lists the market as cards that hide what the club may not see", async () => {
  renderCentre();
  const rival = await screen.findByRole("article", { name: "Never Scouted" });
  const own = screen.getByRole("article", { name: "Own Listed" });

  expect(screen.queryByRole("table")).toBeNull();
  expect(within(rival).getAllByText("–")).toHaveLength(2);
  expect(within(rival).queryByText(formatWeeklyWage(REAL_WAGE, "compact"))).toBeNull();
  expect(within(own).getByText("72")).toBeInTheDocument();
  expect(within(own).getByText(formatWeeklyWage(REAL_WAGE, "compact"))).toBeInTheDocument();
});

/**
 * Given the cards view
 * When a card's name is chosen, and a card is right-clicked
 * Then the player opens, and the context menu offers his profile as the table row's does.
 */
it("opens a player from his card and from its context menu", async () => {
  const onSelectPlayer = vi.fn();
  renderCentre(onSelectPlayer);
  const card = await screen.findByRole("article", { name: "Never Scouted" });

  fireEvent.click(within(card).getByRole("button", { name: "Never Scouted" }));
  expect(onSelectPlayer).toHaveBeenCalledWith("unscouted");

  onSelectPlayer.mockClear();
  fireEvent.contextMenu(card);
  fireEvent.click(await screen.findByText("View profile"));
  expect(onSelectPlayer).toHaveBeenCalledWith("unscouted");
});

/**
 * Given a player with a pending bid from the club, in the Offers tab's cards
 * When his card is read
 * Then it shows the offer's fee and its offer-history button.
 */
it("shows a player's offers and the history button on his card in the Offers view", async () => {
  renderCentre();
  fireEvent.click(await screen.findByRole("button", { name: "Offers (1)" }));

  const card = await screen.findByRole("article", { name: "Offered Player" });
  expect(within(card).getByText(new RegExp(formatExactMoney(700001)))).toBeInTheDocument();
  expect(within(card).getByRole("button", { name: "View offer history" })).toBeInTheDocument();
});

/**
 * Given the table view
 * When the player picks Cards in the view switch
 * Then the market becomes cards.
 */
it("switches the market to cards from the view switch", async () => {
  setListView("table");
  renderCentre();
  await screen.findByRole("table");

  fireEvent.click(screen.getByRole("radio", { name: "Cards" }));

  expect(await screen.findAllByRole("article")).not.toHaveLength(0);
  expect(screen.queryByRole("table")).toBeNull();
});
