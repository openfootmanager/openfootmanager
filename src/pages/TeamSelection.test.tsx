import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, LeagueData } from "../store/gameStore";
import { useGameStore } from "../store/gameStore";
import { createPlayer, createTeam } from "../test-utils/factories";
import { ThemeProvider } from "../context/ThemeContext";
import TeamSelection from "./TeamSelection";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

vi.mock("react-router-dom", () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock("../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../test-utils/positionTable");
  return cannedPositionsModule();
});

vi.mock("../services/portraitService", () => ({
  prewarmManagerSquadPortraits: vi.fn(),
}));

vi.mock("../store/gameStore", () => ({ useGameStore: vi.fn() }));

// One `t` for every render, as react-i18next gives: a fresh function each time would make anything
// memoised on it (the derived regions) a new value on every render.
const translate = vi.hoisted(
  () => (key: string, params?: Record<string, string | number>) =>
    params?.defaultValue !== undefined ? String(params.defaultValue) : key,
);
const i18nState = vi.hoisted(() => ({ language: "en" }));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({ t: translate, i18n: i18nState }),
}));

// jsdom doesn't implement matchMedia, which ThemeProvider reads on mount.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: query === "(prefers-color-scheme: dark)",
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

/** The backend's facts for team-1 (`get_team_selection_ratings`). */
const CLUB_FACTS = {
  team_id: "team-1",
  average_rating: 70,
  players: [],
  stadium_name: "Ground",
  stadium_capacity: 20_000,
  transfer_budget: 5_000_000,
  weekly_wage_budget: 100_000,
  balance: 1_000_000,
  colors: { primary: "#112233", secondary: "#ffffff" },
  founded_year: 1900,
  competitions: [],
  key_players: [],
  reputation_key: "teamSelect.repAverage",
  opening_expectation: { target_position: 10, label_key: "teamSelect.expectations.midTable" },
};

function league(overrides: Partial<LeagueData> = {}): LeagueData {
  return {
    id: "epl",
    name: "Premier League",
    season: 2025,
    scope: "Domestic",
    kind: "League",
    region_id: "europe",
    country_id: "GB",
    participant_ids: ["team-1"],
    priority: 1,
    fixtures: [],
    standings: [],
    ...overrides,
  };
}

function buildGameState(): GameStateData {
  return {
    teams: [
      createTeam({ id: "team-1", name: "Alpha FC", short_name: "ALP", country: "GB" }),
      createTeam({ id: "team-2", name: "Bravo Town", short_name: "BRV", country: "GB" }),
    ],
    players: [createPlayer({ id: "p1", team_id: "team-1" })],
    competitions: [league({ participant_ids: ["team-1", "team-2"] })],
    league: null,
    clock: { current_date: "2026-07-01", start_date: "2026-07-01" },
    manager: { first_name: "Sam", last_name: "Boss" },
  } as unknown as GameStateData;
}

/** The page's header bar: a plain lookup, as it is the only `header` the screen draws. */
function pageHeader(): HTMLElement {
  const header = document.querySelector("header");
  if (!header) throw new Error("the page has no header");
  return header;
}

function renderPage() {
  render(
    <ThemeProvider>
      <TeamSelection />
    </ThemeProvider>,
  );
}

describe("TeamSelection", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(useGameStore).mockReturnValue({
      gameState: buildGameState(),
      setGameState: vi.fn(),
      setGameActive: vi.fn(),
    } as unknown as ReturnType<typeof useGameStore>);
  });

  /**
   * Given a world with two clubs in one league
   * When the screen opens
   * Then the steps, the country and league rail and the clubs' table are drawn; no club is chosen,
   *      so the card says so and Manage is disabled; and the simulation dialog is closed.
   */
  it("opens with no club chosen and Manage disabled", () => {
    renderPage();

    expect(screen.getByRole("heading", { level: 1, name: "teamSelect.title" })).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "teamSelect.steps.label" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "teamSelect.rail.label" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Alpha FC" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(screen.getByRole("button", { name: "Bravo Town" })).toBeInTheDocument();

    expect(screen.getByRole("button", { name: "teamSelect.manageNone" })).toBeDisabled();
    // A disabled button says why: no club is chosen yet.
    expect(
      screen.getByRole("button", { name: "teamSelect.manageNone" }),
    ).toHaveAccessibleDescription("teamSelect.card.emptyTitle");
    const card = screen.getByRole("complementary", { name: "teamSelect.card.label" });
    expect(within(card).getByText("teamSelect.card.emptyTitle")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  /**
   * Given the screen with no club chosen
   * When a club is pressed in the table
   * Then Manage names that club and is enabled, and the chosen-club card shows it.
   */
  it("enables Manage and shows the card once a club is chosen", () => {
    renderPage();

    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));

    expect(screen.getByRole("button", { name: "Alpha FC" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(within(pageHeader()).getByRole("button", { name: "teamSelect.manage" })).toBeEnabled();
    const card = screen.getByRole("complementary", { name: "teamSelect.card.label" });
    expect(within(card).getByRole("heading", { name: "Alpha FC" })).toBeInTheDocument();
    expect(within(card).queryByText("teamSelect.card.emptyTitle")).toBeNull();
  });

  /**
   * Given a club chosen
   * When the chosen-club card is read
   * Then it ends with a Manage button named like the header's, and before a club is chosen the card
   *      has none (it only says to choose one).
   */
  it("puts a second Manage button at the foot of the chosen club's card", () => {
    renderPage();
    const card = screen.getByRole("complementary", { name: "teamSelect.card.label" });
    expect(within(card).queryByRole("button", { name: "teamSelect.manage" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));

    expect(screen.getAllByRole("button", { name: "teamSelect.manage" })).toHaveLength(2);
    expect(within(card).getByRole("button", { name: "teamSelect.manage" })).toBeEnabled();
    expect(card.lastElementChild).toContainElement(
      within(card).getByRole("button", { name: "teamSelect.manage" }),
    );
  });

  /**
   * Given a club chosen and the backend ready to start the career
   * When the header's Manage is pressed, and in a fresh screen the card's
   * Then each asks the backend for the same career: the same club with the same regions and
   *      competitions, one request each (one handler, not two copies).
   */
  it("starts the same career from the header's Manage and the card's", async () => {
    const requestsFrom = async (pressManage: () => void) => {
      vi.mocked(invoke).mockReset();
      vi.mocked(invoke).mockImplementation(async (command) =>
        command === "select_team"
          ? ({ manager: { display_name: "Sam Boss" } } as never)
          : undefined,
      );
      renderPage();
      fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));
      pressManage();
      await waitFor(() =>
        expect(vi.mocked(invoke).mock.calls.filter(([c]) => c === "select_team")).toHaveLength(1),
      );
      const request = vi.mocked(invoke).mock.calls.find(([c]) => c === "select_team");
      cleanup();
      return request;
    };

    const fromHeader = await requestsFrom(() =>
      fireEvent.click(within(pageHeader()).getByRole("button", { name: "teamSelect.manage" })),
    );
    const fromCard = await requestsFrom(() =>
      fireEvent.click(
        within(screen.getByRole("complementary", { name: "teamSelect.card.label" })).getByRole(
          "button",
          { name: "teamSelect.manage" },
        ),
      ),
    );

    expect(fromHeader?.[1]).toMatchObject({ teamId: "team-1" });
    expect(fromCard).toEqual(fromHeader);
  });

  /**
   * Given the career being started from the card's Manage (the request has not answered)
   * When both buttons are read, and the header's is pressed too
   * Then both read as confirming and are disabled, and no second request is made.
   */
  it("shows both Manage buttons as confirming while the career starts", async () => {
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === "select_team" ? await new Promise<never>(() => {}) : undefined,
    );
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));

    fireEvent.click(
      within(screen.getByRole("complementary", { name: "teamSelect.card.label" })).getByRole(
        "button",
        { name: "teamSelect.manage" },
      ),
    );

    await waitFor(() =>
      expect(screen.getAllByRole("button", { name: "teamSelect.confirming" })).toHaveLength(2),
    );
    for (const button of screen.getAllByRole("button", { name: "teamSelect.confirming" })) {
      expect(button).toBeDisabled();
      fireEvent.click(button);
    }
    expect(vi.mocked(invoke).mock.calls.filter(([c]) => c === "select_team")).toHaveLength(1);
  });

  /**
   * Given a club chosen
   * When the keyboard's tab stops are listed in the order the page offers them
   * Then the card's Manage is the first stop after the clubs table's last (nothing sits between the
   *      table and it), and the header's Manage, which is the other way to the same career, is
   *      before the table.
   */
  it("reaches the card's Manage right after the clubs table in tab order", () => {
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));
    const stops = [
      ...document.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ];
    const table = screen.getByRole("table", { name: "teamSelect.table.label" });
    const lastInTable = stops.filter((stop) => table.contains(stop)).at(-1) as HTMLElement;
    const cardManage = within(
      screen.getByRole("complementary", { name: "teamSelect.card.label" }),
    ).getByRole("button", { name: "teamSelect.manage" });
    const headerManage = within(pageHeader()).getByRole("button", {
      name: "teamSelect.manage",
    });

    expect(stops[stops.indexOf(lastInTable) + 1]).toBe(cardManage);
    expect(stops.indexOf(headerManage)).toBeLessThan(
      stops.indexOf(table.querySelector("button") as HTMLElement),
    );
  });

  /**
   * Given a club chosen, and the backend has not yet named its facts (its own competitions and
   *       regions)
   * When the header is read, and then the facts arrive
   * Then Simulation waits, so the dialog never shows the club as having no competitions of its own;
   *      with the facts it is available.
   */
  it("makes the simulation dialog wait for the chosen club's facts", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));
    expect(screen.getByRole("button", { name: "teamSelect.simulation" })).toBeDisabled();

    vi.mocked(invoke).mockResolvedValue({ clubs: [CLUB_FACTS] });
    cleanup();
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "teamSelect.simulation" })).toBeEnabled(),
    );
  });

  /**
   * Given the simulation settings closed
   * When the header's Simulation button is pressed
   * Then the simulation dialog opens, naming the home region and the regions and competitions to
   *      simulate; Cancel closes it, and the settings are as they were.
   */
  it("opens the simulation dialog from the header, and Cancel closes it", () => {
    renderPage();

    fireEvent.click(screen.getByRole("button", { name: "teamSelect.simulation" }));

    const dialog = screen.getByRole("dialog", { name: "teamSelect.scope.title" });
    expect(within(dialog).getByRole("switch", { name: "Europe" })).toBeChecked();

    fireEvent.click(within(dialog).getByRole("button", { name: "common.cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  /**
   * Given the simulation dialog open, with a setting changed (the only competition unchecked)
   * When it is dismissed, by Cancel, by the close button, or by Escape, and then reopened
   * Then the setting is as it was (the competition is checked again), and each time focus returned
   *      to the Simulation button that opened it: a dismissal discards the draft, however it is made.
   */
  it.each([
    ["Cancel", () => fireEvent.click(screen.getByRole("button", { name: "common.cancel" }))],
    [
      "the close button",
      () => fireEvent.click(screen.getByRole("button", { name: "common.close" })),
    ],
    ["Escape", () => fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" })],
  ])("discards the draft and returns focus when dismissed by %s", (_name, dismiss) => {
    renderPage();
    const opener = screen.getByRole("button", { name: "teamSelect.simulation" });
    opener.focus();
    fireEvent.click(opener);
    const competition = () => screen.getByRole("checkbox", { name: "teamSelect.scope.simulate" });
    expect(competition()).toBeChecked();
    fireEvent.click(competition());
    expect(competition()).not.toBeChecked();

    dismiss();

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(opener).toHaveFocus();
    fireEvent.click(opener);
    expect(competition()).toBeChecked();
  });

  /**
   * Given the dialog open
   * When Apply is pressed
   * Then the dialog closes and the screen goes on with the settings applied.
   */
  it("closes the dialog when Apply is pressed", () => {
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "teamSelect.simulation" }));

    fireEvent.click(screen.getByRole("button", { name: "teamSelect.scope.apply" }));

    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
