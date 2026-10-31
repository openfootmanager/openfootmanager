import { render, screen, within } from "@testing-library/react";
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
const MARKET = [unscouted, scouted];

// What the backend's projection lets the club see of each of them.
const FIGURES: Record<string, VisibleFigures> = {
  unscouted: { ovr: null, wage: { kind: "hidden" } },
  scouted: { ovr: 68, wage: { kind: "approximate", weekly: 12000 } },
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
 * Given a scouted player: his scout saw a 68, and the page puts his wage at about €12,000 a week
 * When the transfer centre lists him
 * Then his row shows the 68 and "about" that wage.
 */
it("lists a scouted player with his scouted rating and his wage as about", async () => {
  renderCentre();
  const row = await rowOf("Once Scouted");

  expect(within(row).getByText("68")).toBeInTheDocument();
  expect(within(row).getByText(`about ${formatWeeklyWage(12000, "compact")}`)).toBeInTheDocument();
});
