import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { createInstance } from "i18next";
import en from "../../i18n/locales/en.json";
import TransfersTab from "./TransfersTab";
import { invoke } from "@tauri-apps/api/core";
import { formatExactMoney, formatWeeklyWage } from "../../lib/helpers";
import type { VisibleFigures } from "../../lib/playerKnowledge";
import type { GameStateData, PlayerData } from "../../store/gameStore";
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

// Another club's players on €12,345 a week: whatever the screen shows of that wage, it must have
// been sent by the market page, never read off the full player.
const REAL_WAGE = 12345;
const unscouted = createPlayer({
  id: "unscouted",
  full_name: "Never Scouted",
  team_id: "team-2",
  transfer_listed: true,
  wage: REAL_WAGE,
});
const scouted = createPlayer({
  id: "scouted",
  full_name: "Once Scouted",
  team_id: "team-2",
  transfer_listed: true,
  wage: REAL_WAGE,
});
const rival = createPlayer({
  id: "rival",
  full_name: "Division Rival",
  team_id: "team-2",
  transfer_listed: true,
  wage: REAL_WAGE,
});
// The manager's own player, listed for transfer.
const ownListed = createPlayer({
  id: "own-listed",
  full_name: "Own Listed",
  team_id: "team-1",
  transfer_listed: true,
  wage: REAL_WAGE,
});
const loanUnscouted = createPlayer({
  id: "loan-unscouted",
  full_name: "Loan Unscouted",
  team_id: "team-2",
  transfer_listed: false,
  loan_listed: true,
  wage: REAL_WAGE,
});
const loanScouted = createPlayer({
  id: "loan-scouted",
  full_name: "Loan Scouted",
  team_id: "team-2",
  transfer_listed: false,
  loan_listed: true,
  wage: REAL_WAGE,
});
// The user's own player, with another club's loan offer waiting for an answer.
const ownWithLoanOffer = createPlayer({
  id: "own-loan-offer",
  full_name: "Own Wanted",
  team_id: "team-1",
  wage: REAL_WAGE,
  loan_offers: [
    {
      id: "incoming-loan",
      from_team_id: "team-2",
      parent_team_id: "team-1",
      start_date: "2026-08-01",
      end_date: "2027-01-28",
      wage_contribution_pct: 100,
      buy_option_fee: null,
      status: "Pending",
      date: "2026-08-01",
    },
  ],
});
const MARKET = [unscouted, scouted, rival, ownListed, loanUnscouted, loanScouted];

// What the backend's projection lets the club see of each of them (`KnowledgeView::of_player`):
// a rating only for its own player and a division rival, so a scouted player's is unknown until a
// scout's estimate exists; a wage exact for its own player, about the figure once scouted.
const FIGURES: Record<string, VisibleFigures> = {
  unscouted: { ovr: null, wage: { kind: "hidden" } },
  scouted: { ovr: null, wage: { kind: "approximate", weekly: 12000 } },
  rival: { ovr: 71, wage: { kind: "hidden" } },
  "own-listed": { ovr: 72, wage: { kind: "exact", weekly: REAL_WAGE } },
  "loan-unscouted": { ovr: null, wage: { kind: "hidden" } },
  "loan-scouted": { ovr: null, wage: { kind: "approximate", weekly: 12000 } },
  "own-loan-offer": { ovr: 72, wage: { kind: "exact", weekly: REAL_WAGE } },
};

function pageWithFigures(players: PlayerData[]) {
  const page = marketPageOf(players);
  return {
    ...page,
    items: page.items.map((item) => ({ ...item, ...FIGURES[item.id] })),
  };
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (command: string, payload?: unknown) =>
    command === "get_players_page"
      ? pageWithFigures(
          playersOfScope(payload, {
            market: MARKET,
            user_listed: [],
            with_offers: [ownWithLoanOffer],
          }),
        )
      : {},
  );
});

const renderCentre = () =>
  render(
    <TransfersTab
      gameState={createGameState([...MARKET, ownWithLoanOffer])}
      onSelectPlayer={vi.fn()}
      onSelectTeam={vi.fn()}
      onGameUpdate={vi.fn()}
    />,
  );

async function rowOf(name: string): Promise<HTMLElement> {
  const row = (await screen.findByText(name)).closest("tr");
  if (row === null) throw new Error(`no row holds ${name}`);
  return row;
}

/**
 * Given another club's listed player whom the club has not scouted, and the market page that
 *       sends neither his rating nor his wage
 * When the transfer centre lists him
 * Then his row shows "–" for both, and nothing of his real wage.
 */
it("lists an unscouted rival with a dash for his rating and his wage", async () => {
  renderCentre();
  const row = await rowOf("Never Scouted");

  expect(within(row).getAllByText("–")).toHaveLength(2);
  expect(within(row).queryByText(formatWeeklyWage(REAL_WAGE, "compact"))).not.toBeInTheDocument();
});

/**
 * Given a scouted player, whose wage the page puts at about €12,000 a week and whose rating it
 *       does not send (a report carries no rating until a scout's estimate exists)
 * When the transfer centre lists him
 * Then his row shows "about" that wage, and "–" for his rating.
 */
it("lists a scouted player with his wage as about and no rating", async () => {
  renderCentre();
  const row = await rowOf("Once Scouted");

  expect(within(row).getByText(`about ${formatWeeklyWage(12000, "compact")}`)).toBeInTheDocument();
  expect(within(row).getAllByText("–")).toHaveLength(1);
});

/**
 * Given the manager's own listed player, whose rating (72) and exact wage the page sends
 * When the transfer centre lists him
 * Then his row shows both.
 */
it("lists the club's own player with his rating and exact wage", async () => {
  renderCentre();
  const row = await rowOf("Own Listed");

  expect(within(row).getByText("72")).toBeInTheDocument();
  expect(within(row).getByText(formatWeeklyWage(REAL_WAGE, "compact"))).toBeInTheDocument();
});

/**
 * Given a listed player at a club in the manager's division, whose rating of 71 the page sends and
 *       whose wage it hides
 * When the transfer centre lists him
 * Then his row shows the 71, and "–" for his wage.
 */
it("lists a division rival with his rating and a dash for his wage", async () => {
  renderCentre();
  const row = await rowOf("Division Rival");

  expect(within(row).getByText("71")).toBeInTheDocument();
  expect(within(row).getAllByText("–")).toHaveLength(1);
  expect(within(row).queryByText(formatWeeklyWage(REAL_WAGE, "compact"))).not.toBeInTheDocument();
});

/** The row holding `name` as the screen shows it now. */
function rowNow(name: string): HTMLElement {
  const row = screen.getByText(name).closest("tr");
  if (row === null) throw new Error(`no row holds ${name}`);
  return row;
}

/** The centre, shown for one game and then for the game `change` makes of it, the next page
 *  answered by `nextPage` (pending or failed in these tests). */
async function changeGameWithTheCentreOpen(
  change: (before: GameStateData) => GameStateData,
  nextPage: () => Promise<unknown>,
) {
  const before = createGameState(MARKET);
  const { rerender } = render(
    <TransfersTab
      gameState={before}
      onSelectPlayer={vi.fn()}
      onSelectTeam={vi.fn()}
      onGameUpdate={vi.fn()}
    />,
  );
  expect(within(await rowOf("Division Rival")).getByText("71")).toBeInTheDocument();
  vi.mocked(invoke).mockImplementation(async (command: string) =>
    command === "get_players_page" ? nextPage() : {},
  );
  rerender(
    <TransfersTab
      gameState={change(before)}
      onSelectPlayer={vi.fn()}
      onSelectTeam={vi.fn()}
      onGameUpdate={vi.fn()}
    />,
  );
}

/** The same career after the manager took a job at another club. */
const tookAnotherJob = (before: GameStateData): GameStateData => ({
  ...before,
  manager: { ...before.manager, team_id: "team-3" },
});

/** The centre after the manager took a job at another club. */
const changeClubWithTheCentreOpen = (nextPage: () => Promise<unknown>) =>
  changeGameWithTheCentreOpen(tookAnotherJob, nextPage);

/**
 * Given the centre open on a division rival whose rating, 71, the manager's club could see
 * When the manager takes a job at another club and the new club's page is still on its way
 * Then the rival's row shows "–" for his rating: the old club's answer is not the new club's.
 */
it("hides the old club's figures while the new club's page is on its way", async () => {
  await changeClubWithTheCentreOpen(() => new Promise(() => {}));

  await waitFor(() => expect(within(rowNow("Division Rival")).queryByText("71")).toBeNull());
  expect(within(rowNow("Division Rival")).getAllByText("–")).toHaveLength(2);
});

/**
 * Given the centre open on a division rival whose rating, 71, the manager's club could see
 * When the manager takes a job at another club and the new club's page fails to load
 * Then the rival's row keeps showing "–" for his rating, not the old club's answer.
 */
it("keeps the old club's figures hidden when the new club's page fails", async () => {
  await changeClubWithTheCentreOpen(() => Promise.reject(new Error("backend unavailable")));

  await waitFor(() => expect(within(rowNow("Division Rival")).queryByText("71")).toBeNull());
  expect(within(rowNow("Division Rival")).getAllByText("–")).toHaveLength(2);
});

/** The same day, after the manager's own listed player was sold to another club. */
const soldOwnListed = (before: GameStateData): GameStateData => ({
  ...before,
  players: before.players.map((player) =>
    player.id === ownListed.id ? { ...player, team_id: "team-2" } : player,
  ),
});

/**
 * Given the centre open on the club's own listed player, his rating (72) and exact wage showing
 * When he is sold to another club that same day, and the page after the sale is still on its way
 * Then his row shows "–" for both: they were his old club's to see, and the career, the date and
 *      the manager's club have not moved, so only his club tells the old answer is not his now.
 */
it("hides a sold player's figures while the page after the sale is on its way", async () => {
  await changeGameWithTheCentreOpen(soldOwnListed, () => new Promise(() => {}));

  await waitFor(() => expect(within(rowNow("Own Listed")).queryByText("72")).toBeNull());
  expect(within(rowNow("Own Listed")).getAllByText("–")).toHaveLength(2);
});

/**
 * Given the centre open on the club's own listed player, his rating (72) and exact wage showing
 * When he is sold to another club that same day, and the page after the sale fails to load
 * Then his row keeps showing "–" for both, not his old club's answer.
 */
it("keeps a sold player's figures hidden when the page after the sale fails", async () => {
  await changeGameWithTheCentreOpen(soldOwnListed, () =>
    Promise.reject(new Error("backend unavailable")),
  );

  await waitFor(() => expect(within(rowNow("Own Listed")).queryByText("72")).toBeNull());
  expect(within(rowNow("Own Listed")).getAllByText("–")).toHaveLength(2);
});

/**
 * Given the unscouted rival on the market
 * When the user opens his deal workspace from his row
 * Then its figures show "–" for his rating and his current wage, as his row did.
 */
it("opens an unscouted rival's deal workspace with his rating and wage hidden", async () => {
  renderCentre();
  const row = await rowOf("Never Scouted");

  fireEvent.click(within(row).getByRole("button", { name: /^bid$/i }));

  const workspace = await screen.findByRole("dialog", { name: "Never Scouted" });
  expect(within(workspace).getByText("OVR").nextElementSibling).toHaveTextContent(/^–$/);
  expect(within(workspace).getByText("Current Wage").nextElementSibling).toHaveTextContent(/^–$/);
  expect(within(workspace).queryByText(formatWeeklyWage(REAL_WAGE))).not.toBeInTheDocument();
});

/**
 * Given another club's loan-listed player whom the club has not scouted
 * When the user opens a loan offer for him
 * Then the share of his wage the club would cover is written "–", not worked out from his real wage.
 */
it("hides the wage share of a loan offer for an unscouted player", async () => {
  renderCentre();
  const row = await rowOf("Loan Unscouted");

  fireEvent.click(within(row).getByRole("button", { name: /^loan offer$/i }));

  expect(
    await screen.findByText("Your club would cover 100% of the player's wages: –."),
  ).toBeInTheDocument();
});

/**
 * Given the scouted player on the market, whose wage the page puts at about €12,000 and whose
 *       rating it does not send
 * When the user opens his deal workspace from his row
 * Then its figures show "about €12,000/wk" and "–" for his rating, as his row did.
 */
it("opens a scouted player's deal workspace with his wage as about and no rating", async () => {
  renderCentre();
  const row = await rowOf("Once Scouted");

  fireEvent.click(within(row).getByRole("button", { name: /^bid$/i }));

  const workspace = await screen.findByRole("dialog", { name: "Once Scouted" });
  expect(within(workspace).getByText("OVR").nextElementSibling).toHaveTextContent(/^–$/);
  expect(within(workspace).getByText("Current Wage").nextElementSibling).toHaveTextContent(
    `about ${formatWeeklyWage(12000)}`,
  );
});

/**
 * Given a scouted loan-listed player whose wage the club puts at about €12,000 a week
 * When the user opens a loan offer for him
 * Then the share the club would cover is the whole of that visible wage, written as "about".
 */
it("writes the wage share of a loan offer for a scouted player as about", async () => {
  renderCentre();
  const row = await rowOf("Loan Scouted");

  fireEvent.click(within(row).getByRole("button", { name: /^loan offer$/i }));

  expect(
    await screen.findByText(
      `Your club would cover 100% of the player's wages: about ${formatExactMoney(12000)}.`,
    ),
  ).toBeInTheDocument();
});

/**
 * Given the user's own player on €12,345 a week, with another club's loan offer waiting
 * When the user counters that offer from the Offers view
 * Then the counter form works the wage share out from his exact wage.
 */
it("writes the exact wage share when countering a loan offer for the club's own player", async () => {
  renderCentre();
  fireEvent.click(screen.getByRole("button", { name: /offers/i }));

  fireEvent.click(await screen.findByRole("button", { name: /counter loan offer/i }));

  expect(
    await screen.findByText(
      `Your club would cover 100% of the player's wages: ${formatExactMoney(REAL_WAGE)}.`,
    ),
  ).toBeInTheDocument();
});
