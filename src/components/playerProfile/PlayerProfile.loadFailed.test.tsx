import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { createInstance } from "i18next";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import en from "../../i18n/locales/en.json";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { createPlayer, createSeasonContext, createTeam } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

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

// He is the club's own player, at no contract risk. His wage, the opinion of his potential and his
// position ratings all go through their real hooks, against a backend that can fail.
vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => () => "stable",
}));
vi.mock("../../hooks/usePlayerKnowledge", () => ({
  usePlayerKnowledge: () => () => ({ kind: "own" }),
}));
vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const emptyStatsOverview = {
  percentileEligible: false,
  metrics: {
    shots: { total: 0, per90: null, percentile: null },
    shotsOnTarget: { total: 0, per90: null, percentile: null },
    passes: { completed: 0, attempted: 0, accuracy: null, percentile: null },
    tacklesWon: { total: 0, per90: null, percentile: null },
    interceptions: { total: 0, per90: null, percentile: null },
    foulsCommitted: { total: 0, per90: null, percentile: null },
  },
};

function gameWith(player: PlayerData): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: { current_date: "2026-09-15T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [createTeam()],
    players: [player],
    staff: [],
    messages: [],
    news: [],
    league: { id: "league-1", name: "League", season: 1, fixtures: [], standings: [] },
    scouting_assignments: [],
    board_objectives: [],
  };
}

type Command = "get_visible_wage" | "get_potential_opinion" | "get_position_ratings";

/**
 * The backend answering the profile. `failing` names the command whose first request fails; every
 * other request answers, `get_visible_wage` with an exact 12,000.
 */
function backendWhere(failing: Command | null, answers: Partial<Record<Command, unknown>> = {}) {
  const failedOnce = new Set<string>();
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === failing && !failedOnce.has(command)) {
      failedOnce.add(command);
      throw "be.error.noActiveGameSession";
    }
    if (command in answers) return answers[command as Command];
    if (command === "get_visible_wage") return { kind: "exact", weekly: 12_000 };
    if (command === "get_player_stats_overview") return emptyStatsOverview;
    if (command === "get_player_match_history") return [];
    return null;
  });
}

const calls = (command: string) =>
  vi.mocked(invoke).mock.calls.filter(([called]) => called === command).length;

/** Opens the profile on the tab that holds what a test is about: the opinion and the positions are
 *  on Attributes, the wage on Contract. */
const renderProfile = (tab: "Overview" | "Attributes" | "Contract" = "Attributes") => {
  const player = createPlayer({ id: "player-1", team_id: "team-1" });
  render(<PlayerProfile player={player} gameState={gameWith(player)} isOwnClub />);
  fireEvent.click(screen.getByRole("tab", { name: tab }));
};

const opinion = {
  low: 76,
  high: 84,
  band: "A",
  certainty: "fairly_sure",
  source: { kind: "coaches" },
  wonderkid: false,
};

const ratings = {
  source: { kind: "live" },
  rows: [
    {
      position: "CentralMidfielder",
      rating: 77,
      familiarity: "natural",
      deployed: false,
      weights: [],
      key_attributes: [],
    },
  ],
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

describe("a profile answer that fails to load", () => {
  /**
   * Given the backend holds no opinion of him and no ratings (it answers null), and his wage
   * When his profile opens and every answer has arrived
   * Then nothing reads "Couldn't load": a real "nothing known" is not a failure
   */
  it("does not mistake nothing known for a failure", async () => {
    backendWhere(null);
    // The wage answers last, a task after the two nulls, so its figure on screen proves both null
    // answers were already in when nothing reads "Couldn't load".
    const answer = vi.mocked(invoke).getMockImplementation();
    vi.mocked(invoke).mockImplementation(async (command: string, args?: unknown) =>
      command === "get_visible_wage"
        ? new Promise((resolve) => setTimeout(() => resolve({ kind: "exact", weekly: 12_000 }), 0))
        : answer?.(command, args as never),
    );

    renderProfile("Overview");

    const wageRow = screen.getByText("Wage").parentElement as HTMLElement;
    await waitFor(() => expect(wageRow).toHaveTextContent(/12/));
    expect(calls("get_potential_opinion")).toBe(1);
    expect(calls("get_position_ratings")).toBe(1);
    expect(screen.queryByText("Couldn't load")).not.toBeInTheDocument();
  });

  /**
   * Given the request for the opinion of his potential fails once
   * When his profile opens, and the manager retries
   * Then it reads "Couldn't load" with a retry, never as no opinion; after the retry the opinion
   *      shows
   */
  it("offers a retry for a failed opinion, then shows it", async () => {
    backendWhere("get_potential_opinion", { get_potential_opinion: opinion });

    renderProfile();

    // On the Attributes tab and in the Overview's Details, which is a hidden panel now.
    expect(await screen.findAllByText("Couldn't load")).toHaveLength(2);
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByRole("region", { name: "Coaches' opinion" })).toBeInTheDocument();
    expect(screen.queryAllByText("Couldn't load")).toHaveLength(0);
    expect(calls("get_potential_opinion")).toBe(2);
  });

  /**
   * Given the request for his position ratings fails once
   * When his profile opens, and the manager retries
   * Then it reads "Couldn't load" with a retry; after the retry the tab's dots and highlight show
   */
  it("offers a retry for failed position ratings, then uses them", async () => {
    backendWhere("get_position_ratings", { get_position_ratings: ratings });

    renderProfile();

    // The Overview's attributes card and the Attributes tab each say so.
    expect(await screen.findAllByText("Couldn't load")).toHaveLength(2);
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText(/what the game weighs most for a/)).toBeInTheDocument();
    expect(calls("get_position_ratings")).toBe(2);
  });

  /**
   * Given the request for his wage fails once
   * When his profile opens on the Contract tab
   * Then the contract row reads "Couldn't load", never "–", with a retry; retrying clears it
   */
  it("retries a failed wage from the contract row", async () => {
    backendWhere("get_visible_wage");

    renderProfile("Overview");

    const wageRow = screen.getByText("Wage").parentElement as HTMLElement;
    const rowRetry = await within(wageRow).findByRole("button", { name: "Retry" });
    expect(rowRetry).toHaveAccessibleDescription("Retry loading the wage");
    expect(wageRow).toHaveTextContent("Couldn't load");

    fireEvent.click(rowRetry);

    await waitFor(() => expect(within(wageRow).queryByRole("button")).not.toBeInTheDocument());
    expect(screen.queryByText("Couldn't load")).not.toBeInTheDocument();
    expect(wageRow).not.toHaveTextContent("–");
    expect(calls("get_visible_wage")).toBe(2);
  });

  /**
   * Given the request for his wage fails once
   * When his profile opens on the Contract tab
   * Then the wage retry is named by the text it shows, so a voice-control user can say what they
   *      see (WCAG 2.5.3), and says what it retries as its description
   */
  it("names the wage retry by the text it shows", async () => {
    backendWhere("get_visible_wage");

    renderProfile("Overview");

    const retries = await screen.findAllByRole("button", { description: "Retry loading the wage" });
    expect(retries.length).toBe(1);
    for (const retry of retries) {
      expect(retry).toHaveAccessibleName(retry.textContent ?? "");
    }
  });
});
