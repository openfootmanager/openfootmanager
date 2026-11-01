import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetPositionGroupsCache } from "../../hooks/usePositionGroups";
import { resetRatingTiersCache } from "../../hooks/useRatingTiers";
import { useSettingsStore } from "../../store/settingsStore";
import { resetMatchCommands } from "./matchCommands";
import { makeGameState, makeSnapshot } from "./matchScreenFixtures";
import PenaltyShootoutScreen from "./PenaltyShootoutScreen";
import type { EnginePlayerData, MatchEvent, MatchSnapshot, ShootoutOrder } from "./types";

const mockedInvoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: mockedInvoke }));
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (k: string, params?: Record<string, unknown>) =>
      params && Object.keys(params).length > 0 ? `${k}:${Object.values(params).join(",")}` : k,
  }),
}));
// The screen shows a refused command through the backend error helper; this stands in for its
// translation so a test can tell the helper's text from the raw key.
vi.mock("../../utils/backendI18n", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../utils/backendI18n")>()),
  resolveBackendError: (error: unknown) => `translated:${String(error)}`,
}));

const base = makeSnapshot();
const alice = base.home_team.players[0];

function player(
  id: string,
  name: string,
  overrides: Partial<EnginePlayerData> = {},
): EnginePlayerData {
  return { ...alice, id, name, ...overrides };
}

/** The home order as the engine first gives it: Alice (the taker), Carol, Dan, then Eve. */
const AUTO_HOME = ["p1", "p3", "p4", "p5"];
/** Beta FC finished with three, so both sides take three and the fourth home player sits out. */
const KICKERS = 3;

const kick = (side: "Home" | "Away", playerId: string): MatchEvent => ({
  minute: 120,
  event_type: "ShootoutGoal",
  side,
  zone: "Box",
  player_id: playerId,
  secondary_player_id: null,
});

/**
 * Alpha FC (home: Alice p1, Carol p3, Dan p4, Eve p5) against Beta FC (away: Bob p2, Finn p6,
 * Gus p7), level after extra time, before the first kick. The user manages Alpha FC unless the
 * orders say otherwise.
 */
function pickerSnapshot(
  overrides: {
    events?: MatchEvent[];
    homeOrder?: ShootoutOrder | null;
    awayOrder?: ShootoutOrder | null;
  } = {},
): MatchSnapshot {
  return {
    ...base,
    phase: "PenaltyShootout",
    home_score: 1,
    away_score: 1,
    events: overrides.events ?? [],
    home_team: {
      ...base.home_team,
      players: [
        player("p1", "Alice", { position: "Forward", shooting: 81, composure: 64, condition: 55 }),
        player("p3", "Carol", { position: "Midfielder" }),
        player("p4", "Dan", { position: "Defender" }),
        player("p5", "Eve", { position: "Goalkeeper" }),
      ],
    },
    away_team: {
      ...base.away_team,
      players: [base.away_team.players[0], player("p6", "Finn"), player("p7", "Gus")],
    },
    home_set_pieces: { ...base.home_set_pieces, penalty_taker: "p1" },
    penalty_shootout: {
      home_taken: 0,
      away_taken: 0,
      home_scored: 0,
      away_scored: 0,
      sudden_death: false,
      home_order:
        overrides.homeOrder === undefined
          ? { player_ids: AUTO_HOME, kickers: KICKERS }
          : overrides.homeOrder,
      away_order: overrides.awayOrder ?? null,
    },
  };
}

function renderScreen(
  snapshot: MatchSnapshot,
  options: {
    userSide?: "Home" | "Away" | null;
    onSnapshotUpdate?: (s: MatchSnapshot) => void;
  } = {},
) {
  const userSide = options.userSide === undefined ? "Home" : options.userSide;
  return render(
    <PenaltyShootoutScreen
      snapshot={snapshot}
      gameState={makeGameState()}
      userSide={userSide}
      isSpectator={userSide === null}
      importantEvents={[]}
      onSnapshotUpdate={options.onSnapshotUpdate ?? (() => {})}
      onImportantEvent={() => {}}
      onFullTime={() => {}}
    />,
  );
}

const freshSettings = useSettingsStore.getState();

/** What the engine answers: the tables the badges read, a kick per step, the order it was sent. */
function answerAsTheEngine(snapshot: MatchSnapshot) {
  mockedInvoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    switch (command) {
      case "get_rating_tiers":
      case "get_position_groups":
        return [];
      case "step_live_match":
        return [{ minute: 120, phase: "PenaltyShootout", events: [], is_finished: false }];
      case "get_match_snapshot":
        return snapshot;
      case "apply_match_command":
        return { ...snapshot, current_minute: 121, args };
      default:
        return null;
    }
  });
}

const callsTo = (command: string) => mockedInvoke.mock.calls.filter(([name]) => name === command);

const flush = () =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });

const orderList = () => screen.getByRole("list", { name: "match.shootout.orderHeading" });

/** Each row's grip, top to bottom, by its accessible name. */
const gripNames = () =>
  within(orderList())
    .getAllByRole("button")
    .map((grip) => grip.getAttribute("aria-label"));

/** The grip of the named player, wherever he is in the order. */
const gripOf = (name: string) =>
  within(orderList()).getByRole("button", { name: new RegExp(`:${name}(,|$)`) });

/** Which entry of the list holds the sits-out line, counting the rows above it. */
const linePlace = () => {
  const lineEntry = within(orderList()).getByRole("separator").closest("li");
  const entries: Array<Element | null> = Array.from(orderList().children);
  return entries.indexOf(lineEntry);
};

const announced = (text: string) => screen.getByText(text, { selector: "[role='status']" });

/** The home column of the kick sheet, round by round. */
const homeSheetColumn = () => {
  const sheet = screen.getByRole("table", { name: "match.shootout.sheetCaption" });
  return within(sheet)
    .getAllByRole("row")
    .slice(1)
    .map((row) => within(row).getAllByRole("cell")[0].textContent);
};

beforeEach(() => {
  vi.useFakeTimers();
  mockedInvoke.mockReset();
  resetMatchCommands();
  resetRatingTiersCache();
  resetPositionGroupsCache();
  useSettingsStore.setState(freshSettings, true);
  answerAsTheEngine(pickerSnapshot());
});

afterEach(() => {
  vi.useRealTimers();
});

describe("PenaltyShootoutScreen: when the order picker shows", () => {
  /**
   * Given the user's side has an order, before the first kick
   * When the shootout screen opens
   * Then it asks for the penalty order, listing the players in the engine's order with the
   * sits-out line after the third, and the kick sheet previews the first five kicks from it.
   */
  it("lists the order with the line after the kickers, and previews it on the sheet", () => {
    renderScreen(pickerSnapshot());

    expect(screen.getByRole("heading", { name: "match.shootout.orderHeading" })).toBeVisible();
    expect(gripNames()).toEqual([
      "match.shootout.moveKicker:Alice,1,3",
      "match.shootout.moveKicker:Carol,2,3",
      "match.shootout.moveKicker:Dan,3,3",
      "match.shootout.moveSitter:Eve",
    ]);
    const line = within(orderList()).getByRole("separator", {
      name: "match.shootout.sitsOutLine:1,3",
    });
    expect(linePlace()).toBe(3);
    expect(line).not.toHaveAttribute("tabindex");

    expect(homeSheetColumn()).toEqual([
      "1Alicematch.shootout.toCome",
      "2Carolmatch.shootout.toCome",
      "3Danmatch.shootout.toCome",
      "4Alicematch.shootout.toCome",
      "5Carolmatch.shootout.toCome",
    ]);
    expect(screen.getByText("match.shootout.opponentOrderHidden:Beta FC")).toBeInTheDocument();
  });

  /**
   * Given as many kickers as players
   * When the picker lists them
   * Then there is no sits-out line: everyone kicks.
   */
  it("draws no line when everyone kicks", () => {
    renderScreen(pickerSnapshot({ homeOrder: { player_ids: AUTO_HOME, kickers: 4 } }));

    expect(gripNames()).toHaveLength(4);
    expect(within(orderList()).queryByRole("separator")).not.toBeInTheDocument();
  });

  /**
   * Given Alice is the designated penalty taker, with shooting 81, composure 64 and condition 55
   * When her row is drawn
   * Then it carries the pen-taker badge and those three values, and no other row has the badge.
   */
  it("shows the taker badge, shooting, composure and condition on each row", () => {
    renderScreen(pickerSnapshot());

    const [aliceRow, carolRow] = within(orderList()).getAllByRole("listitem");
    expect(within(aliceRow).getByText("match.shootout.penTaker")).toBeInTheDocument();
    expect(
      within(aliceRow).getByText("common.namedRating:common.attributes.shooting,81"),
    ).toBeInTheDocument();
    expect(
      within(aliceRow).getByText("common.namedRating:common.attributes.composure,64"),
    ).toBeInTheDocument();
    expect(within(aliceRow).getByRole("progressbar", { name: "common.condition" })).toHaveAttribute(
      "aria-valuenow",
      "55",
    );
    expect(within(carolRow).queryByText("match.shootout.penTaker")).not.toBeInTheDocument();
  });

  /**
   * Given the player chose to skip the picker in Settings
   * When the shootout screen opens before the first kick
   * Then it goes straight to the kicks, in the auto order.
   */
  it("skips the picker when the setting is on", () => {
    useSettingsStore.setState({
      settings: { ...freshSettings.settings, skip_shootout_picker: true },
    });
    renderScreen(pickerSnapshot());

    expect(screen.queryByRole("list", { name: "match.shootout.orderHeading" })).toBeNull();
    expect(screen.getByRole("button", { name: "match.shootout.nextKick" })).toBeInTheDocument();
  });

  /**
   * Given a match between two other clubs, where the engine sends no order
   * When the shootout screen opens
   * Then there is nothing to pick and it goes straight to the kicks.
   */
  it("goes straight to the kicks when spectating", () => {
    renderScreen(pickerSnapshot({ homeOrder: null }), { userSide: null });

    expect(screen.queryByRole("list", { name: "match.shootout.orderHeading" })).toBeNull();
    expect(screen.getByRole("button", { name: "match.shootout.nextKick" })).toBeInTheDocument();
  });

  /**
   * Given the user's side but no order for it in the snapshot
   * When the shootout screen opens
   * Then it goes straight to the kicks rather than offering an empty list.
   */
  it("goes straight to the kicks when the user's side has no order", () => {
    renderScreen(pickerSnapshot({ homeOrder: null }));

    expect(screen.queryByRole("list", { name: "match.shootout.orderHeading" })).toBeNull();
    expect(screen.getByRole("button", { name: "match.shootout.nextKick" })).toBeInTheDocument();
  });

  /**
   * Given the first kick has already been taken (the screen reopened mid-shootout)
   * When the shootout screen opens
   * Then the order can no longer change, so it goes straight to the kicks.
   */
  it("goes straight to the kicks once a kick has been taken", () => {
    renderScreen(pickerSnapshot({ events: [kick("Home", "p1")] }));

    expect(screen.queryByRole("list", { name: "match.shootout.orderHeading" })).toBeNull();
    expect(screen.getByRole("button", { name: "match.shootout.nextKick" })).toBeInTheDocument();
  });

  /**
   * Given the picker is showing
   * When time passes
   * Then no kick is taken: the engine does not wait for the order, so the screen must, and the
   * speed control (and its Space shortcut) is not on screen to start it.
   */
  it("takes no kick while the picker shows", async () => {
    renderScreen(pickerSnapshot());

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(callsTo("step_live_match")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: "match.shootout.nextKick" })).toBeNull();
  });
});

describe("PenaltyShootoutScreen: setting the order by keyboard", () => {
  /**
   * Given the order list
   * When the player tabs to it and presses the down arrow
   * Then the focus moves to the next player and the order stays as it was: the list is one tab
   * stop, with only the focused player's grip in the tab order.
   */
  it("moves the focus between players with the arrow keys", () => {
    renderScreen(pickerSnapshot());
    const tabStops = () =>
      within(orderList())
        .getAllByRole("button")
        .filter((grip) => grip.getAttribute("tabindex") === "0");
    expect(tabStops()).toEqual([gripOf("Alice")]);

    gripOf("Alice").focus();
    fireEvent.keyDown(gripOf("Alice"), { key: "ArrowDown" });

    expect(gripOf("Carol")).toHaveFocus();
    expect(tabStops()).toEqual([gripOf("Carol")]);
    expect(gripNames()[0]).toBe("match.shootout.moveKicker:Alice,1,3");
  });

  /**
   * Given Carol, second in the order
   * When the player lifts her with Space, moves her up and drops her with Space
   * Then she kicks first, each step is announced, the lifted row says it is moving, and the
   * sheet's preview follows.
   */
  it("lifts, moves and drops a player, announcing each step", () => {
    renderScreen(pickerSnapshot());
    gripOf("Carol").focus();

    fireEvent.keyDown(gripOf("Carol"), { key: " " });
    expect(announced("match.shootout.liftedKick:Carol,2,3")).toBeInTheDocument();
    const lifted = gripOf("Carol").closest("li") as HTMLElement;
    expect(within(lifted).getByText("match.shootout.moving")).toBeInTheDocument();

    fireEvent.keyDown(gripOf("Carol"), { key: "ArrowUp" });
    expect(announced("match.shootout.movedKick:Carol,1,3")).toBeInTheDocument();
    expect(gripOf("Carol")).toHaveFocus();

    fireEvent.keyDown(gripOf("Carol"), { key: "Enter" });
    expect(announced("match.shootout.droppedKick:Carol,1,3")).toBeInTheDocument();
    expect(screen.queryByText("match.shootout.moving")).not.toBeInTheDocument();
    expect(gripNames()).toEqual([
      "match.shootout.moveKicker:Carol,1,3",
      "match.shootout.moveKicker:Alice,2,3",
      "match.shootout.moveKicker:Dan,3,3",
      "match.shootout.moveSitter:Eve",
    ]);
    expect(homeSheetColumn()[0]).toBe("1Carolmatch.shootout.toCome");
  });

  /**
   * Given Dan, the third kicker
   * When the player lifts him, moves him down past the line and drops him
   * Then Dan sits out and Eve takes the third kick: the line stays after three, and the move
   * across it is announced as sitting out.
   */
  it("swaps who kicks when a player is moved across the line", () => {
    renderScreen(pickerSnapshot());
    gripOf("Dan").focus();

    fireEvent.keyDown(gripOf("Dan"), { key: " " });
    fireEvent.keyDown(gripOf("Dan"), { key: "ArrowDown" });
    expect(announced("match.shootout.movedSitsOut:Dan")).toBeInTheDocument();
    fireEvent.keyDown(gripOf("Dan"), { key: " " });

    expect(announced("match.shootout.droppedSitsOut:Dan")).toBeInTheDocument();
    expect(gripNames()).toEqual([
      "match.shootout.moveKicker:Alice,1,3",
      "match.shootout.moveKicker:Carol,2,3",
      "match.shootout.moveKicker:Eve,3,3",
      "match.shootout.moveSitter:Dan",
    ]);
    expect(linePlace()).toBe(3);
  });

  /**
   * Given Alice lifted and carried down two places
   * When the player presses Escape
   * Then she is back first, as before she was lifted, and that is announced.
   */
  it("puts a lifted player back where she started on Escape", () => {
    renderScreen(pickerSnapshot());
    gripOf("Alice").focus();

    fireEvent.keyDown(gripOf("Alice"), { key: " " });
    fireEvent.keyDown(gripOf("Alice"), { key: "ArrowDown" });
    fireEvent.keyDown(gripOf("Alice"), { key: "ArrowDown" });
    expect(gripNames()[2]).toBe("match.shootout.moveKicker:Alice,3,3");
    fireEvent.keyDown(gripOf("Alice"), { key: "Escape" });

    expect(announced("match.shootout.putBackKick:Alice,1,3")).toBeInTheDocument();
    expect(gripNames()).toEqual([
      "match.shootout.moveKicker:Alice,1,3",
      "match.shootout.moveKicker:Carol,2,3",
      "match.shootout.moveKicker:Dan,3,3",
      "match.shootout.moveSitter:Eve",
    ]);
    expect(gripOf("Alice")).toHaveFocus();
  });

  /**
   * Given the auto order, Alice first and Carol second
   * When the player drags Carol's row onto Alice's
   * Then Carol kicks first and Alice second, and the drop is announced, as by keyboard.
   */
  it("moves a player by dragging his row onto another's", () => {
    renderScreen(pickerSnapshot());
    const carolRow = gripOf("Carol").closest("li") as HTMLElement;
    const aliceRow = gripOf("Alice").closest("li") as HTMLElement;

    fireEvent.dragStart(carolRow, { dataTransfer: { setData: () => {}, effectAllowed: "" } });
    fireEvent.dragOver(aliceRow, { dataTransfer: { dropEffect: "" } });
    fireEvent.drop(aliceRow, { dataTransfer: {} });

    expect(gripNames()[0]).toBe("match.shootout.moveKicker:Carol,1,3");
    expect(gripNames()[1]).toBe("match.shootout.moveKicker:Alice,2,3");
    expect(
      screen.getByText(/^match\.shootout\.droppedKick:Carol/, { selector: "[role='status']" }),
    ).toBeInTheDocument();
  });

  /**
   * Given an order the player changed
   * When he chooses Use auto order
   * Then the engine's first order is back, and that is announced.
   */
  it("restores the auto order", () => {
    renderScreen(pickerSnapshot());
    gripOf("Carol").focus();
    fireEvent.keyDown(gripOf("Carol"), { key: " " });
    fireEvent.keyDown(gripOf("Carol"), { key: "ArrowUp" });
    fireEvent.keyDown(gripOf("Carol"), { key: " " });

    fireEvent.click(screen.getByRole("button", { name: "match.shootout.useAutoOrder" }));

    expect(gripNames()[0]).toBe("match.shootout.moveKicker:Alice,1,3");
    expect(gripNames()[1]).toBe("match.shootout.moveKicker:Carol,2,3");
    expect(announced("match.shootout.autoOrderRestored")).toBeInTheDocument();
  });
});

describe("PenaltyShootoutScreen: starting the shootout", () => {
  /**
   * Given the auto order, untouched
   * When the player starts the shootout
   * Then no order is sent (the engine has it already), the kicks begin, and the focus moves to
   * the kick sheet.
   */
  it("starts with the auto order without sending it", async () => {
    renderScreen(pickerSnapshot());

    fireEvent.click(screen.getByRole("button", { name: "match.shootout.startShootout" }));
    await flush();

    expect(callsTo("apply_match_command")).toHaveLength(0);
    expect(screen.queryByRole("list", { name: "match.shootout.orderHeading" })).toBeNull();
    expect(screen.getByRole("heading", { name: "match.shootout.kickSheet" })).toHaveFocus();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(callsTo("step_live_match")).toHaveLength(1);
  });

  /**
   * Given the player moved Carol to first
   * When he starts the shootout
   * Then exactly that order is sent for his side, the screen takes the snapshot it answers with,
   * and only then do the kicks begin.
   */
  it("sends a changed order, then starts the kicks", async () => {
    const onSnapshotUpdate = vi.fn();
    renderScreen(pickerSnapshot(), { onSnapshotUpdate });
    gripOf("Carol").focus();
    fireEvent.keyDown(gripOf("Carol"), { key: " " });
    fireEvent.keyDown(gripOf("Carol"), { key: "ArrowUp" });
    fireEvent.keyDown(gripOf("Carol"), { key: " " });

    fireEvent.click(screen.getByRole("button", { name: "match.shootout.startShootout" }));
    await flush();

    expect(callsTo("apply_match_command")).toEqual([
      [
        "apply_match_command",
        {
          command: { SetShootoutOrder: { side: "Home", player_ids: ["p3", "p1", "p4", "p5"] } },
          careerToken: 0,
        },
      ],
    ]);
    expect(onSnapshotUpdate).toHaveBeenCalledWith(expect.objectContaining({ current_minute: 121 }));
    expect(callsTo("step_live_match")).toHaveLength(0);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(callsTo("step_live_match")).toHaveLength(1);
  });

  /**
   * Given the user manages the away side
   * When he moves Gus to first and starts the shootout
   * Then the order is sent for the away side, with the away players.
   */
  it("sends the away side's order for an away manager", async () => {
    renderScreen(
      pickerSnapshot({
        homeOrder: null,
        awayOrder: { player_ids: ["p2", "p6", "p7"], kickers: 3 },
      }),
      { userSide: "Away" },
    );
    expect(screen.getByText("match.shootout.opponentOrderHidden:Alpha FC")).toBeInTheDocument();
    gripOf("Gus").focus();
    fireEvent.keyDown(gripOf("Gus"), { key: " " });
    fireEvent.keyDown(gripOf("Gus"), { key: "ArrowUp" });
    fireEvent.keyDown(gripOf("Gus"), { key: "ArrowUp" });
    fireEvent.keyDown(gripOf("Gus"), { key: " " });

    fireEvent.click(screen.getByRole("button", { name: "match.shootout.startShootout" }));
    await flush();

    expect(callsTo("apply_match_command")[0]?.[1]).toEqual({
      command: { SetShootoutOrder: { side: "Away", player_ids: ["p7", "p2", "p6"] } },
      careerToken: 0,
    });
  });

  /**
   * Given the engine refuses the order
   * When the player starts the shootout
   * Then the refusal is shown in words as an alert, the picker stays, and no kick is taken.
   */
  it("shows a refused order and stays on the picker", async () => {
    // Tauri rejects a refused command with the error string itself.
    mockedInvoke.mockImplementation((command: string) => {
      if (command === "apply_match_command") {
        return Promise.reject("be.error.liveMatch.shootoutOrderNotNow");
      }
      const tables = command === "get_rating_tiers" || command === "get_position_groups";
      return Promise.resolve(tables ? [] : null);
    });
    renderScreen(pickerSnapshot());
    gripOf("Carol").focus();
    fireEvent.keyDown(gripOf("Carol"), { key: " " });
    fireEvent.keyDown(gripOf("Carol"), { key: "ArrowUp" });
    fireEvent.keyDown(gripOf("Carol"), { key: " " });

    fireEvent.click(screen.getByRole("button", { name: "match.shootout.startShootout" }));
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(
      "translated:be.error.liveMatch.shootoutOrderNotNow",
    );
    expect(orderList()).toBeInTheDocument();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(callsTo("step_live_match")).toHaveLength(0);
  });
});
