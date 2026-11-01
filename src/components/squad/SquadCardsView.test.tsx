import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { formatDate } from "../../lib/dateFormatting";
import { formatWeeklyWage } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import { createPlayer, createTeam } from "../../test-utils/factories";
import SquadRosterView from "./SquadRosterView";

// What the backend's contract warning stages read as on a badge, by player id; a test says which
// players are at risk.
const riskLevels = vi.hoisted(() => ({
  current: {} as Record<string, "critical" | "warning" | "stable">,
}));

// Whether the backend's position table has arrived; a test says it has not.
const positionTableState = vi.hoisted(() => ({ arrived: true }));

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  const { createPositionTable } = await import("../../lib/positionTable");
  const canned = cannedPositionsModule();
  const notArrived = createPositionTable([]);
  return {
    ...canned,
    usePositionTable: () => {
      const arrived = canned.usePositionTable();
      return positionTableState.arrived ? arrived : notArrived;
    },
  };
});

vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => (playerId: string) => riskLevels.current[playerId] ?? "stable",
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: string | Record<string, unknown>) => {
      const params = typeof options === "object" ? options : undefined;
      if (key === "playerProfile.injuryDaysShort") return `${String(params?.count ?? "")}d`;
      if (key === "playerProfile.daysRemaining") return `${String(params?.count ?? "")} days`;
      if (key.startsWith("common.injuries.")) return String(params?.defaultValue ?? key);
      return typeof options === "string" ? options : key;
    },
    i18n: { language: "en" },
  }),
  initReactI18next: { type: "3rdParty", init: () => {} },
}));

const makePlayer = (
  id: string,
  position: string,
  overrides: Partial<PlayerData> = {},
): PlayerData =>
  createPlayer({
    id,
    match_name: id.toUpperCase(),
    full_name: `Player ${id}`,
    date_of_birth: "1998-01-01",
    position,
    natural_position: position,
    attributes: { ...createPlayer().attributes, handling: 60, reflexes: 60 },
    condition: 100,
    morale: 80,
    team_id: "team1",
    wage: 1000,
    market_value: 100000,
    contract_end: "2028-06-30",
    match_day_eligible: true,
    ...overrides,
  });

const STARTER_IDS = ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"];

/** Eleven starters in a 4-4-2 and one fit defender on the bench, "Bench DEF". */
function makeSquad(): PlayerData[] {
  return [
    makePlayer("gk1", "Goalkeeper", {
      wage: 4200,
      ovr: 74,
      condition: 92,
      condition_status: { band: "Fresh", label_key: "be.condition.fresh", tone: "good" },
    }),
    makePlayer("d1", "Center Back"),
    makePlayer("d2", "Defender"),
    makePlayer("d3", "Defender"),
    makePlayer("d4", "Defender"),
    makePlayer("m1", "Midfielder"),
    makePlayer("m2", "Midfielder"),
    makePlayer("m3", "Midfielder"),
    makePlayer("m4", "Midfielder"),
    makePlayer("f1", "Forward"),
    makePlayer("f2", "Forward"),
    makePlayer("d5", "Defender", { match_name: "Bench DEF" }),
  ];
}

const team = createTeam({ id: "team1", name: "Test FC", starting_xi_ids: STARTER_IDS });

function setListView(view: "table" | "cards") {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, list_view: view },
  });
}

function renderRoster(
  players: PlayerData[] = makeSquad(),
  onSelectPlayer = vi.fn(),
  rosterTeam = team,
) {
  render(
    <SquadRosterView
      players={players}
      team={rosterTeam}
      clockDate="2026-08-01"
      onSelectPlayer={onSelectPlayer}
      onMutationComplete={vi.fn()}
    />,
  );
  return { onSelectPlayer };
}

const menuItemNames = () =>
  screen.getAllByRole("menuitem").map((item) => item.textContent?.trim() ?? "");

/** The cards' players in the order drawn, read from each card's name. */
const cardNames = () =>
  screen
    .getAllByRole("article")
    .map((card) => within(card).getByRole("heading", { level: 3 }).textContent);

const firstCardName = () => cardNames()[0];

describe("SquadRosterView cards", () => {
  beforeEach(() => {
    riskLevels.current = {};
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
    setListView("cards");
  });

  afterEach(() => setListView("table"));

  /**
   * Given list_view is "cards"
   * When the squad's roster renders
   * Then each first-team player is a card and there is no table.
   */
  it("draws the roster as one card per player when the list view is cards", () => {
    renderRoster();

    expect(screen.getAllByRole("article")).toHaveLength(12);
    expect(screen.getByRole("article", { name: "Bench DEF" })).toBeInTheDocument();
    expect(screen.queryByRole("table")).toBeNull();
  });

  /**
   * Given list_view is "table"
   * When the squad's roster renders
   * Then it is the roster table, with no cards.
   */
  it("draws the roster as its table when the list view is table", () => {
    setListView("table");
    renderRoster();

    expect(screen.getByRole("table", { name: "squad.title" })).toBeInTheDocument();
    expect(screen.queryByRole("article")).toBeNull();
  });

  /**
   * Given the roster's table
   * When the manager picks Cards on the view switch
   * Then the roster becomes cards and the choice is saved as list_view, for every list.
   */
  it("switches to cards from the view switch and saves the choice", async () => {
    setListView("table");
    renderRoster();

    await act(async () => {
      fireEvent.click(screen.getByRole("radio", { name: "common.listViewCards" }));
    });

    expect(screen.getAllByRole("article")).toHaveLength(12);
    expect(screen.queryByRole("table")).toBeNull();
    expect(invoke).toHaveBeenCalledWith(
      "save_settings",
      expect.objectContaining({ settings: expect.objectContaining({ list_view: "cards" }) }),
    );
  });

  /**
   * Given a starting keeper rated 74 whom the backend rates fresh, on 4,200 a week, whose
   *       contract ends in June 2028 and is in its critical stage
   * When his card renders
   * Then it names him and shows his rating, his condition's band word, the critical contract chip,
   *      his wage and the month his contract ends.
   */
  it("shows a player's rating, condition, contract stage, wage and contract end", () => {
    riskLevels.current = { gk1: "critical" };
    renderRoster();

    const card = screen.getByRole("article", { name: "GK1" });
    expect(within(card).getByText("74")).toBeInTheDocument();
    expect(within(card).getByText("be.condition.fresh")).toBeInTheDocument();
    expect(within(card).getByText("finances.contractRiskCritical")).toBeInTheDocument();
    expect(within(card).getByText(formatWeeklyWage(4200, "compact"))).toBeInTheDocument();
    expect(
      within(card).getByText(formatDate("2028-06-30", "en", { month: "short", year: "numeric" })),
    ).toBeInTheDocument();
  });

  /**
   * Given a starter and a fit bench player
   * When their cards render
   * Then the starter's card says in words that he is in the Starting XI, and the bench player's
   *      does not.
   */
  it("marks a starter's card with the Starting XI badge and not a bench player's", () => {
    renderRoster();

    expect(
      within(screen.getByRole("article", { name: "GK1" })).getByText("squad.startingXi"),
    ).toBeInTheDocument();
    expect(
      within(screen.getByRole("article", { name: "Bench DEF" })).queryByText("squad.startingXi"),
    ).toBeNull();
  });

  /**
   * Given an injured bench forward
   * When his card renders
   * Then it names his injury, and a fit player's card names none.
   */
  it("shows an injured player's injury on his card", () => {
    renderRoster([
      ...makeSquad(),
      makePlayer("inj", "Forward", {
        match_name: "Hurt Forward",
        injury: { name: "Knee bruise", days_remaining: 10 },
        match_day_eligible: false,
      }),
    ]);

    const hurt = screen.getByRole("article", { name: "Hurt Forward" });
    expect(within(hurt).getByRole("img", { name: /Knee bruise/ })).toBeInTheDocument();
    expect(
      within(screen.getByRole("article", { name: "Bench DEF" })).queryByRole("img", {
        name: /Knee bruise/,
      }),
    ).toBeNull();
  });

  /**
   * Given a card
   * When its name's button is clicked
   * Then the player's profile opens.
   */
  it("opens the player from his card's name", () => {
    const { onSelectPlayer } = renderRoster();

    const card = screen.getByRole("article", { name: "Bench DEF" });
    fireEvent.click(within(card).getByRole("button", { name: "Bench DEF" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("d5");
  });

  /**
   * Given the bench defender's row menu in the table
   * When the view switches to cards and his card is right-clicked
   * Then the card's menu offers exactly the row's items, in the row's order.
   */
  it("gives a card the same context menu as the player's row", async () => {
    setListView("table");
    renderRoster();
    fireEvent.contextMenu(screen.getByText("Bench DEF").closest("tr") as HTMLTableRowElement);
    const rowItems = menuItemNames();
    expect(rowItems).toContain("squad.makeStarter");

    await act(async () => {
      fireEvent.click(screen.getByRole("radio", { name: "common.listViewCards" }));
    });
    fireEvent.contextMenu(screen.getByRole("article", { name: "Bench DEF" }));

    expect(menuItemNames()).toEqual(rowItems);
  });

  /**
   * Given the bench defender's card, reached without a mouse
   * When its actions button is pressed
   * Then the card's menu opens with the row's items, the button says the menu is open, and the
   *      player's profile does not open.
   */
  it("opens a card's menu from its actions button", () => {
    const { onSelectPlayer } = renderRoster();
    const card = screen.getByRole("article", { name: "Bench DEF" });
    const actions = within(card).getByRole("button", { name: "common.playerActions" });
    expect(actions).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(actions);

    expect(menuItemNames()).toContain("squad.makeStarter");
    expect(actions).toHaveAttribute("aria-expanded", "true");
    expect(onSelectPlayer).not.toHaveBeenCalled();
  });

  /**
   * Given a fit bench defender the backend rules out of the match day, an injured forward it rules
   * out too, and a fit defender it doesn't
   * When their cards render
   * Then the first card says he can't be picked; the injured one names only his injury, and the
   *      eligible one says nothing of the kind.
   */
  it("says on his card that a player the backend rules out can't be picked", () => {
    renderRoster([
      ...makeSquad(),
      makePlayer("out", "Defender", { match_name: "Ruled Out", match_day_eligible: false }),
      makePlayer("inj", "Forward", {
        match_name: "Hurt Forward",
        injury: { name: "Knee bruise", days_remaining: 10 },
        match_day_eligible: false,
      }),
    ]);

    const cardSays = (name: string) =>
      within(screen.getByRole("article", { name })).queryByText("squad.notMatchDayEligible");
    expect(cardSays("Ruled Out")).toBeInTheDocument();
    expect(cardSays("Hurt Forward")).toBeNull();
    expect(cardSays("Bench DEF")).toBeNull();
  });

  /**
   * Given the roster drawn before the backend's squad arrives, its players with no match-day
   *       verdict yet (as the game state's raw players are while the squad loads)
   * When their cards render
   * Then none says he can't be picked: an absent verdict means not yet, not no.
   */
  it("says nothing of selection before the backend's verdict arrives", () => {
    renderRoster(makeSquad().map((player) => ({ ...player, match_day_eligible: undefined })));

    expect(screen.getAllByRole("article")).toHaveLength(12);
    expect(screen.queryByText("squad.notMatchDayEligible")).toBeNull();
  });

  /**
   * Given a starting XI in which the goalkeeper and a forward have swapped slots
   * When their cards render
   * Then each card marks the slot he is deployed in as out of position, as his row's badge does,
   *      and a bench player's card and a midfielder's in his own line mark nothing of the kind.
   */
  it("marks a starter deployed out of position on his card", () => {
    const swapped = createTeam({
      id: "team1",
      name: "Test FC",
      starting_xi_ids: ["f1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "gk1", "f2"],
    });
    renderRoster(makeSquad(), vi.fn(), swapped);

    const outOfPosition = (name: string) =>
      within(screen.getByRole("article", { name })).queryAllByText(/, squad\.outOfPosition$/);
    expect(outOfPosition("GK1")).toHaveLength(1);
    expect(outOfPosition("F1")).toHaveLength(1);
    expect(outOfPosition("M1")).toHaveLength(0);
    expect(outOfPosition("Bench DEF")).toHaveLength(0);
  });

  /**
   * Given the bench defender's card, its name's button focused
   * When the Menu key (or Shift+F10) opens the menu, which reports its right-click at 0,0
   * Then the menu opens under the focused button, as a row's does under the row, not in the
   *      viewport's top corner.
   */
  it("opens a card's menu from the keyboard under the focused control", () => {
    renderRoster();
    const card = screen.getByRole("article", { name: "Bench DEF" });
    const name = within(card).getByRole("button", { name: "Bench DEF" });
    name.getBoundingClientRect = () =>
      ({ left: 120, bottom: 340, top: 300, right: 320, width: 200, height: 40 }) as DOMRect;
    name.focus();

    fireEvent.contextMenu(name, { clientX: 0, clientY: 0 });

    expect(screen.getByRole("menu")).toHaveStyle({ left: "120px", top: "340px" });
  });

  /**
   * Given the position table has not arrived
   * When a bench player's card menu and then a starter's are opened
   * Then "Make starter" and "Send to bench" are disabled, as on the row: a lineup change is not
   *      made without the lines.
   */
  it("disables the lineup actions in a card's menu until the position table has loaded", () => {
    positionTableState.arrived = false;
    try {
      renderRoster();

      fireEvent.contextMenu(screen.getByRole("article", { name: "Bench DEF" }));
      expect(screen.getByRole("menuitem", { name: "squad.makeStarter" })).toBeDisabled();

      fireEvent.contextMenu(screen.getByRole("article", { name: "GK1" }));
      expect(screen.getByRole("menuitem", { name: "squad.sendToBench" })).toBeDisabled();
    } finally {
      positionTableState.arrived = true;
    }
  });

  /**
   * Given the cards in their default order, starters first with the keeper on top
   * When the sort field is changed to name, and then the direction is flipped
   * Then the cards run from "Player d1" to "Player m4", and then from "Player m4" back.
   */
  it("reorders the cards by the field and direction the sort control picks", () => {
    renderRoster();
    expect(firstCardName()).toBe("GK1");

    fireEvent.click(screen.getByRole("combobox", { name: "common.sortBy" }));
    fireEvent.click(screen.getByRole("option", { name: "common.name" }));
    expect(firstCardName()).toBe("D1");

    fireEvent.click(screen.getByRole("button", { name: "common.sortAscending" }));
    expect(firstCardName()).toBe("M4");
  });

  /**
   * Given the cards sorted by name, high to low
   * When name is picked again in the sort control
   * Then the order stays: picking the field already chosen does not flip it.
   */
  it("keeps the direction when the field already chosen is picked again", () => {
    renderRoster();
    fireEvent.click(screen.getByRole("combobox", { name: "common.sortBy" }));
    fireEvent.click(screen.getByRole("option", { name: "common.name" }));
    fireEvent.click(screen.getByRole("button", { name: "common.sortAscending" }));
    const before = cardNames();
    expect(firstCardName()).toBe("M4");

    fireEvent.click(screen.getByRole("combobox", { name: "common.sortBy" }));
    fireEvent.click(screen.getByRole("option", { name: "common.name" }));

    expect(firstCardName()).toBe("M4");
    expect(cardNames()).toEqual(before);
  });

  /**
   * Given the cards view
   * When a search matches nobody
   * Then the roster says the squad's no-players line instead of an empty grid.
   */
  it("says there are no players when the filters leave none", () => {
    renderRoster();

    fireEvent.change(screen.getByRole("textbox", { name: "common.search" }), {
      target: { value: "nobody here" },
    });

    expect(screen.queryByRole("article")).toBeNull();
    expect(screen.getByText("squad.noPlayers")).toBeInTheDocument();
  });
});
