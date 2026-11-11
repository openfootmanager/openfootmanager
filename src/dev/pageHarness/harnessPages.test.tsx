import { act, fireEvent, within } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import { useGameStore } from "../../store/gameStore";
import { readStyleGuideOptions } from "../styleGuide/styleGuideOptions";
import { gameState } from "./fixture.generated";
import {
  HARNESS_PAGES,
  HARNESS_PROFILES,
  type HarnessPage,
  type HarnessProfile,
} from "./harnessPage";
import { profileCase } from "./profileCases";
import { mountPageHarness } from "./mount";

/** Long enough for the dashboard and the page to ask for everything they ask for on mount. */
const SETTLE_MS = 500;

beforeAll(async () => {
  // jsdom has no matchMedia; ThemeProvider asks it for the system theme.
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  });
  await i18nReady;
});

let mounted: Awaited<ReturnType<typeof mountPageHarness>> | null = null;

/**
 * What only each page draws, by role and accessible name (never by a name from the generated club:
 * the names below come from the page's own labels). The shell (the header, the sidebar) draws on
 * every page and proves nothing about the page itself.
 */
const PAGE_CONTENT: Record<HarnessPage, (view: ReturnType<typeof within>) => void> = {
  home: (view) => expect(view.getByRole("heading", { name: "Next Match" })).toBeInTheDocument(),
  squad: (view) => expect(view.getByRole("table", { name: /Squad$/ })).toBeInTheDocument(),
  // The players table has no label of its own; its rows (the header and the fixture's page of players) are the page.
  players: (view) => expect(view.getAllByRole("row").length).toBeGreaterThan(1),
  teams: (view) => expect(view.getByRole("table", { name: /League$/ })).toBeInTheDocument(),
  managers: (view) => expect(view.getByRole("table", { name: "Managers" })).toBeInTheDocument(),
  // The Staff page's own table, labelled by its "My Staff (n)" heading: Home draws many headings,
  // so counting them would pass on the wrong page.
  staff: (view) =>
    expect(view.getByRole("table", { name: /^My Staff \(\d+\)$/ })).toBeInTheDocument(),
  // The profile of the club's own player: his name is the page's heading.
  profile: (view) => expect(view.getAllByRole("heading", { level: 1 }).length).toBeGreaterThan(0),
};

afterEach(async () => {
  // Unmounted before the mocks go: the dashboard's event listeners unregister through them.
  await act(async () => mounted?.unmount());
  mounted = null;
  clearMocks();
  useGameStore.getState().clearGame();
  document.body.innerHTML = "";
});

describe("the page harness's pages", () => {
  /**
   * Given each page the harness can mount, on the fixture career
   * When it has settled
   * Then the page rendered (it did not fall back to the "left the dashboard" alert) and the
   *      fixture answered every command it asked for. A command a page starts asking for with
   *      no answer in fixtureAnswers.ts fails here, in the required frontend job, rather than only
   *      in the layout check, which is not a required job and so can sit red unnoticed.
   *      A command a page asks for later than the settle time is not seen: this guard can miss a
   *      late command, never fail on a good page.
   */
  it.each(Object.keys(HARNESS_PAGES) as HarnessPage[])(
    "asks for no command the fixture cannot answer: %s",
    async (page) => {
      const root = document.createElement("div");
      document.body.append(root);

      // The first render is flushed inside act, so it has committed before the settle timer starts.
      await act(async () => {
        mounted = await mountPageHarness(root, page, readStyleGuideOptions(""), "en");
      });
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, SETTLE_MS));
      });

      // An empty unanswered list means nothing only if the page rendered: a harness that never
      // activated the game draws nothing and asks for nothing.
      expect(useGameStore.getState().hasActiveGame).toBe(true);
      expect(root.querySelector('[role="alert"]')).toBeNull();
      PAGE_CONTENT[page](within(root));
      expect(window.__pageHarness?.unanswered).toEqual([]);
    },
    15_000,
  );
});

/** The text of every row labelled `label`: the label's parent holds the label and its value. */
const rowsLabelled = (view: ReturnType<typeof within>, label: string) =>
  view.getAllByText(label).map((element: HTMLElement) => element.parentElement?.textContent ?? "");

const NO_VALUE = /–$/;

/**
 * What only the club's knowledge of the player decides, as the profile shows it: a page that read
 * every player as unknown (a knowledge answer of {} for all) shows the own and the rival profile
 * like the unknown one, and passes the name and the unanswered checks, so these are the check.
 */
const KNOWLEDGE_SHOWS: Record<HarnessProfile, (view: ReturnType<typeof within>) => void> = {
  // The own player's condition, morale and exact wage.
  own: (view) => {
    // The Attributes tab: the dots for his natural position's weights and the three columns.
    expect(view.getAllByLabelText(/^weight \d of 3$/).length).toBeGreaterThan(0);
    for (const column of ["Technical", "Mental", "Physical"]) {
      expect(view.getAllByText(column, { selector: "h4" })).toHaveLength(1);
    }
    for (const row of rowsLabelled(view, "Condition")) expect(row).toMatch(/\d+/);
    for (const row of rowsLabelled(view, "Morale")) expect(row).toMatch(/\d+%/);
    for (const row of rowsLabelled(view, "Wage")) expect(row).toMatch(/€[\d,]+\/wk/);
    expect(view.queryByText("Attributes Hidden")).toBeNull();
    // He has played: his matches are listed (the others' lists are empty).
    expect(
      view.getAllByText(new RegExp(profileCase("own").recentMatches[0].opponent_name)).length,
    ).toBeGreaterThan(0);
  },
  // The own goalkeeper: Goalkeeping in place of Technical.
  keeper: (view) => {
    for (const row of rowsLabelled(view, "Wage")) expect(row).toMatch(/€[\d,]+\/wk/);
    for (const column of ["Goalkeeper", "Mental", "Physical"]) {
      expect(view.getAllByText(column, { selector: "h4" })).toHaveLength(1);
    }
    expect(view.queryByText("Technical", { selector: "h4" })).toBeNull();
  },
  // The own injured player: the injury banner. The own loaned-out player: the loan banner.
  injured: (view) => {
    expect(view.getByText(/12 days remaining/)).toBeInTheDocument();
    // The control for the loaned case below: a player the club still has shows its opinion by now,
    // so an absence there is not a profile that has not finished loading.
    expect(view.getAllByText("Coaches' opinion").length).toBeGreaterThan(0);
  },
  loaned: (view) => {
    expect(view.getByText(/^On loan at /)).toBeInTheDocument();
    // No opinion of his potential while he is away (the coaches see him no more).
    // Not a failed load either: a failure shows a retry where the opinion would be.
    expect(view.queryByRole("button", { name: "Retry" })).toBeNull();
    expect(view.queryByText("Coaches' opinion")).toBeNull();
  },
  // A division rival: his attributes are known, his condition, morale and wage are not.
  rival: (view) => {
    for (const label of ["Condition", "Morale", "Wage"]) {
      for (const row of rowsLabelled(view, label)) expect(row).toMatch(NO_VALUE);
    }
    expect(view.queryByText("Attributes Hidden")).toBeNull();
  },
  // A scouted player: the report's date and a wage that says it is approximate.
  scouted: (view) => {
    expect(view.getAllByText(/^Scouted on /).length).toBeGreaterThan(0);
    for (const row of rowsLabelled(view, "Wage")) expect(row).toContain("about");
    expect(view.queryByText("Attributes Hidden")).toBeNull();
  },
  // A stranger and a retiree: public facts only.
  unknown: (view) => expectHidden(view),
  // A free agent the club has never scouted: public facts only, and no club to be loaned from.
  freeAgent: (view) => {
    expectHidden(view);
    expect(view.queryByText(/^On loan at /)).toBeNull();
  },
  retired: (view) => {
    expectHidden(view);
    // The card says he has retired, when, and from which club.
    const card = within(
      view.getByRole("region", { name: profileCase("retired").retiredPlayer?.full_name }),
    );
    expect(card.getByText("Retired")).toBeInTheDocument();
    expect(card.getByText(/^Retired on /)).toBeInTheDocument();
    expect(card.getByText(/^Last club: /)).toBeInTheDocument();
    expect(card.queryByRole("button", { name: /scout/i })).toBeNull();
    // The top bar names him, as it names any player (it used to read "Player").
    expect(
      view.getByRole("heading", { name: profileCase("retired").retiredPlayer?.match_name }),
    ).toBeInTheDocument();
  },
};

function expectHidden(view: ReturnType<typeof within>) {
  // The Overview's attributes card and the Attributes tab each say so.
  expect(view.getAllByText("Attributes Hidden")).toHaveLength(2);
  for (const row of rowsLabelled(view, "Wage")) expect(row).toMatch(NO_VALUE);
  expect(view.queryByText(/^Scouted on /)).toBeNull();
}

describe("the player profile's states", () => {
  /**
   * Given each state of the club's knowledge of a player (own, rival, scouted, unknown, retired)
   * When the profile is mounted on it and has settled
   * Then it drew that player (his name is on the page), did not fall back to the "left the
   *      dashboard" alert, and the fixture answered every command it asked for: a state a command
   *      has no answer for fails here, in the required job
   */
  it.each(HARNESS_PROFILES)(
    "shows the %s player and asks for no command the fixture cannot answer",
    async (variant) => {
      const root = document.createElement("div");
      document.body.append(root);

      await act(async () => {
        mounted = await mountPageHarness(root, "profile", readStyleGuideOptions(""), "en", variant);
      });
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, SETTLE_MS));
      });

      const { playerId, retiredPlayer } = profileCase(variant);
      const player = retiredPlayer ?? gameState.players.find((p) => p.id === playerId);
      // A case whose player is nowhere would fall back to the Players tab and match any text.
      const fullName = player?.full_name ?? "";
      expect(fullName).not.toBe("");
      expect(root.querySelector('[role="alert"]')).toBeNull();
      expect(within(root).getAllByText(fullName).length).toBeGreaterThan(0);
      expect(window.__pageHarness?.unanswered).toEqual([]);
      KNOWLEDGE_SHOWS[variant](within(root));
    },
    15_000,
  );
});

describe("the profile's Contract tab", () => {
  const PREVIEWS = ["preview_contract_termination", "preview_renewal_financial_impact"];

  async function mountProfile(variant: HarnessProfile) {
    const root = document.createElement("div");
    document.body.append(root);
    await act(async () => {
      mounted = await mountPageHarness(root, "profile", readStyleGuideOptions(""), "en", variant);
    });
    const settle = async () =>
      act(async () => {
        await new Promise((resolve) => setTimeout(resolve, SETTLE_MS));
      });
    await settle();
    return { root, settle };
  }

  /**
   * Given the club's own player's profile, which opens on the Overview
   * When it has settled, and then the Contract tab is selected
   * Then the two previews the decision sheet shows (what releasing him costs, the club's wage
   *      bill) were not asked for on the Overview and are asked for once the tab is open, and the
   *      fixture answered both
   */
  it("asks for the contract previews only once the Contract tab is open", async () => {
    const { root, settle } = await mountProfile("own");
    const asked = () => window.__pageHarness?.calls ?? [];
    expect(PREVIEWS.filter((command) => asked().includes(command))).toEqual([]);

    fireEvent.click(within(root).getByRole("tab", { name: "Contract" }));
    await settle();

    expect(PREVIEWS.filter((command) => asked().includes(command))).toEqual(PREVIEWS);
    expect(window.__pageHarness?.unanswered).toEqual([]);
  });

  /**
   * Given a rival's player, and a free agent (whose contracts the manager does not hold)
   * When the Contract tab is selected
   * Then no preview is asked for: the backend refuses both for a contract that is not his own
   */
  it.each(["rival", "freeAgent"] as const)(
    "asks for no contract preview about the %s player",
    async (variant) => {
      const { root, settle } = await mountProfile(variant);

      fireEvent.click(within(root).getByRole("tab", { name: "Contract" }));
      await settle();

      expect(
        PREVIEWS.filter((command) => (window.__pageHarness?.calls ?? []).includes(command)),
      ).toEqual([]);
    },
  );
});
