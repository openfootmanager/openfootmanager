import { render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { createInstance } from "i18next";
import en from "../../i18n/locales/en.json";
import TransfersTab from "./TransfersTab";
import { invoke } from "@tauri-apps/api/core";
import { formatWeeklyWage } from "../../lib/helpers";
import type { VisibleFigures } from "../../lib/playerKnowledge";
import type { PlayerData } from "../../store/gameStore";
import { createGameState, createPlayer, marketPageOf } from "./TransfersTab.testFixtures";

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
const MARKET = [unscouted, scouted, rival];

// What the backend's projection lets the club see of each of them (`KnowledgeView::of_player`):
// a rating only for its own player and a division rival, so a scouted player's is unknown until a
// scout's estimate exists; a wage exact for its own player, about the figure once scouted.
const FIGURES: Record<string, VisibleFigures> = {
  unscouted: { ovr: null, wage: { kind: "hidden" } },
  scouted: { ovr: null, wage: { kind: "approximate", weekly: 12000 } },
  rival: { ovr: 71, wage: { kind: "hidden" } },
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
  vi.mocked(invoke).mockImplementation(async (command: string) =>
    command === "get_players_page" ? pageWithFigures(MARKET) : {},
  );
});

const renderCentre = () =>
  render(
    <TransfersTab
      gameState={createGameState(MARKET)}
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

/** The centre, shown for one game and then for the same career after the manager took a job at
 *  another club, the new club's page answered by `nextPage` (pending or failed in these tests). */
async function changeClubWithTheCentreOpen(nextPage: () => Promise<unknown>) {
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
      gameState={{ ...before, manager: { ...before.manager, team_id: "team-3" } }}
      onSelectPlayer={vi.fn()}
      onSelectTeam={vi.fn()}
      onGameUpdate={vi.fn()}
    />,
  );
}

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
