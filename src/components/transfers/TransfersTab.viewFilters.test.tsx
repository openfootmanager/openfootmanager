import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { createInstance } from "i18next";
import en from "../../i18n/locales/en.json";
import TransfersTab from "./TransfersTab";
import { invoke } from "@tauri-apps/api/core";
import type { PlayersPageQuery } from "../../services/playersService";
import type { PlayerData } from "../../store/gameStore";
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

const ownListed = createPlayer({
  id: "own-listed",
  full_name: "Own Listed",
  transfer_listed: true,
});
const bidFor = createPlayer({ id: "bid-for", full_name: "Bid For", team_id: "team-2" });
const onMarket = createPlayer({
  id: "on-market",
  full_name: "On Market",
  team_id: "team-2",
  transfer_listed: true,
});

// What the backend answers each list with, whatever the filters say: these tests are about the
// question the screen asks and what it does with the answer, not about the rules (those are Rust's).
let answers: { market: PlayerData[]; user_listed: PlayerData[]; with_offers: PlayerData[] };

const pagesAsked = (): PlayersPageQuery[] =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([command]) => command === "get_players_page")
    .map(([, payload]) => (payload as { query: PlayersPageQuery }).query);

const lastAsked = () => pagesAsked().slice(-1)[0];

beforeEach(() => {
  answers = { market: [onMarket], user_listed: [ownListed], with_offers: [bidFor] };
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (command: string, payload?: unknown) =>
    command === "get_players_page" ? marketPageOf(playersOfScope(payload, answers)) : {},
  );
});

const renderCentre = () =>
  render(
    <TransfersTab
      gameState={createGameState([ownListed, bidFor, onMarket])}
      onSelectPlayer={vi.fn()}
      onSelectTeam={vi.fn()}
      onGameUpdate={vi.fn()}
    />,
  );

const searchBox = () => screen.getByPlaceholderText("Search by name or nationality...");

/**
 * Given the transfer centre
 * When the user opens My Transfer List and searches
 * Then the list is the backend's user-listed page for that search, with the shared filter
 *      controls, and not the market's "Within budget" chip.
 */
it("filters the club's own transfer list through the backend", async () => {
  renderCentre();
  fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));
  expect(await screen.findByText("Own Listed")).toBeInTheDocument();

  fireEvent.change(searchBox(), { target: { value: "own" } });

  await waitFor(() =>
    expect(lastAsked()).toMatchObject({ scope: "user_listed", status: "all", search: "own" }),
  );
  expect(screen.queryByRole("button", { name: "Within budget" })).not.toBeInTheDocument();
});

/**
 * Given the transfer centre
 * When the user opens Offers and narrows to loans
 * Then the backend is asked for the with-offers page with that status.
 */
it("filters the offers through the backend", async () => {
  renderCentre();
  fireEvent.click(screen.getByRole("button", { name: /offers/i }));
  expect(await screen.findByText("Bid For")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Loan" }));

  await waitFor(() => expect(lastAsked()).toMatchObject({ scope: "with_offers", status: "loan" }));
});

/**
 * Given a search typed on the market and another on the club's list
 * When the user moves between the views
 * Then each view finds its own search as it was left.
 */
it("keeps each view's filters when the user comes back to it", async () => {
  renderCentre();
  fireEvent.change(searchBox(), { target: { value: "market-search" } });
  fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));
  expect(await screen.findByText("Own Listed")).toBeInTheDocument();
  expect(searchBox()).toHaveValue("");
  fireEvent.change(searchBox(), { target: { value: "list-search" } });

  fireEvent.click(screen.getByRole("button", { name: /^players/i }));
  await waitFor(() => expect(searchBox()).toHaveValue("market-search"));
  expect(await screen.findByText("On Market")).toBeInTheDocument();
  expect(lastAsked()).toMatchObject({ scope: "market", search: "market-search" });

  fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));
  await waitFor(() => expect(searchBox()).toHaveValue("list-search"));
});

/**
 * Given the user opens a view whose list has not arrived yet
 * When the previous view's rows are still the last page received
 * Then they are not shown under the new view.
 */
it("does not show the market's rows under another view while its page is on the way", async () => {
  renderCentre();
  expect(await screen.findByText("On Market")).toBeInTheDocument();
  vi.mocked(invoke).mockImplementation(() => new Promise(() => {}) as ReturnType<typeof invoke>);

  fireEvent.click(screen.getByRole("button", { name: /offers/i }));

  expect(screen.queryByText("On Market")).not.toBeInTheDocument();
});

/**
 * Given a filter that leaves nothing on the club's list
 * When the list is empty
 * Then it says nothing matches rather than that no player is listed.
 */
it("says nothing matches, not that nothing is listed, when a filter empties the list", async () => {
  renderCentre();
  fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));
  expect(await screen.findByText("Own Listed")).toBeInTheDocument();
  answers.user_listed = [];

  fireEvent.change(searchBox(), { target: { value: "nobody" } });

  expect(await screen.findByText("No players match your filters.")).toBeInTheDocument();
  expect(screen.queryByText(en.transfers.noPlayersListed)).not.toBeInTheDocument();
});
