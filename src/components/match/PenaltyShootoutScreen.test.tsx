import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import PenaltyShootoutScreen, { KickRow } from "./PenaltyShootoutScreen";
import { makeGameState, makeSnapshot } from "./matchScreenFixtures";
import type { MatchEvent, MatchSnapshot } from "./types";

const mockedInvoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: mockedInvoke }));
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (k: string, params?: Record<string, unknown>) =>
      params && Object.keys(params).length > 0 ? `${k}:${Object.values(params).join(",")}` : k,
  }),
}));

/** The cells' accessible names, left to right. */
const cellNames = (container: HTMLElement = document.body) =>
  within(container)
    .getAllByRole("img")
    .map((cell) => cell.getAttribute("aria-label"));

describe("KickRow", (): void => {
  it("shows a cell to come for every untaken kick", (): void => {
    render(<KickRow label="Home" kicks={[]} maxRounds={5} />);
    expect(cellNames()).toEqual([1, 2, 3, 4, 5].map((n) => `match.shootout.kickToCome:${n}`));
  });

  /**
   * Given a side that missed its first penalty and scored its second
   * When its row is drawn
   * Then it reads miss, then goal, in the order the kicks were taken, not the goals first.
   */
  it("draws each kick in the order it was taken", (): void => {
    render(<KickRow label="Home" kicks={[false, true]} maxRounds={5} />);
    expect(cellNames()).toEqual([
      "match.shootout.kickMissed:1",
      "match.shootout.kickScored:2",
      "match.shootout.kickToCome:3",
      "match.shootout.kickToCome:4",
      "match.shootout.kickToCome:5",
    ]);
  });

  /**
   * Given kicks taken and to come
   * When a screen reader reads the row
   * Then the row is named for its club and each cell says which kick it is and how it went.
   */
  it("names the row and each kick for a screen reader", (): void => {
    render(<KickRow label="Away FC" kicks={[true, false, true]} maxRounds={5} />);
    const row = screen.getByRole("group", { name: "Away FC" });
    expect(
      within(row).getByRole("img", { name: "match.shootout.kickScored:1" }),
    ).toBeInTheDocument();
    expect(
      within(row).getByRole("img", { name: "match.shootout.kickMissed:2" }),
    ).toBeInTheDocument();
    expect(
      within(row).getByRole("img", { name: "match.shootout.kickToCome:4" }),
    ).toBeInTheDocument();
  });

  it("draws a cell for every kick taken past maxRounds (sudden death)", (): void => {
    render(
      <KickRow label="Home" kicks={[true, true, false, true, false, true, false]} maxRounds={5} />,
    );
    expect(cellNames()).toHaveLength(7);
    expect(cellNames().filter((name) => name?.includes("kickToCome"))).toHaveLength(0);
  });
});

const kick = (
  side: "Home" | "Away",
  event_type: "ShootoutGoal" | "ShootoutMiss",
  player_id: string,
): MatchEvent => ({
  minute: 120,
  event_type,
  side,
  zone: "Box",
  player_id,
  secondary_player_id: null,
});

/** Alpha FC (home, Alice "p1") against Beta FC (away, Bob "p2"), level 1-1 after extra time. */
function shootoutSnapshot(
  events: MatchEvent[],
  overrides: Partial<MatchSnapshot> = {},
): MatchSnapshot {
  const kicks = events.filter((e) => e.event_type.startsWith("Shootout"));
  const homeKicks = kicks.filter((e) => e.side === "Home");
  const awayKicks = kicks.filter((e) => e.side === "Away");
  return {
    ...makeSnapshot(),
    phase: "PenaltyShootout",
    home_score: 1,
    away_score: 1,
    events,
    penalty_shootout: {
      home_taken: homeKicks.length,
      away_taken: awayKicks.length,
      home_scored: homeKicks.filter((e) => e.event_type === "ShootoutGoal").length,
      away_scored: awayKicks.filter((e) => e.event_type === "ShootoutGoal").length,
      sudden_death: homeKicks.length >= 5 && awayKicks.length >= 5,
    },
    ...overrides,
  };
}

function renderScreen(snapshot: MatchSnapshot, overrides: { onFullTime?: () => void } = {}) {
  return render(
    <PenaltyShootoutScreen
      snapshot={snapshot}
      gameState={makeGameState()}
      userSide="Home"
      isSpectator={false}
      importantEvents={[]}
      onSnapshotUpdate={() => {}}
      onImportantEvent={() => {}}
      onFullTime={overrides.onFullTime ?? (() => {})}
    />,
  );
}

describe("PenaltyShootoutScreen", (): void => {
  beforeEach((): void => {
    // The screen steps the shootout on a timer; held still, it draws only the snapshot it was given.
    vi.useFakeTimers();
    mockedInvoke.mockReset();
  });

  afterEach((): void => {
    vi.useRealTimers();
  });

  /**
   * Given a home goal in extra time, then a shootout where the home side missed its first kick
   * and scored its second
   * When the screen draws the score band
   * Then the home club's row reads miss, then goal, as the kick events happened, without the
   * extra-time goal, which shares the event log but is no kick.
   */
  it("draws the home row from that side's kick events in order", (): void => {
    renderScreen(
      shootoutSnapshot([
        { ...kick("Home", "ShootoutGoal", "p1"), minute: 112, event_type: "Goal" },
        kick("Home", "ShootoutMiss", "p1"),
        kick("Home", "ShootoutGoal", "p1"),
      ]),
    );

    const homeRow = screen.getByRole("group", { name: "Alpha FC" });
    expect(cellNames(homeRow).slice(0, 2)).toEqual([
      "match.shootout.kickMissed:1",
      "match.shootout.kickScored:2",
    ]);
  });

  /**
   * Given round 1 taken (Alice missed, Bob scored) and Alice scoring in round 2
   * When the screen draws the kick sheet
   * Then each round's row names the kicker and the result in that club's column, and round 2,
   * the one under way, is marked as the current step.
   */
  it("writes every kicker and result on the kick sheet, round by round", (): void => {
    renderScreen(
      shootoutSnapshot([
        kick("Home", "ShootoutMiss", "p1"),
        kick("Away", "ShootoutGoal", "p2"),
        kick("Home", "ShootoutGoal", "p1"),
      ]),
    );

    const sheet = screen.getByRole("table", { name: "match.shootout.sheetCaption" });
    const headers = within(sheet)
      .getAllByRole("columnheader")
      .map((cell) => cell.textContent);
    expect(headers).toEqual(["match.shootout.kickColumn", "Alpha FC", "Beta FC"]);

    const [round1, round2] = within(sheet).getAllByRole("row").slice(1);
    const round1Cells = within(round1)
      .getAllByRole("cell")
      .map((cell) => cell.textContent);
    expect(round1Cells).toEqual(["Alicematch.shootout.missed", "Bobmatch.shootout.scored"]);
    const round2Cells = within(round2)
      .getAllByRole("cell")
      .map((cell) => cell.textContent);
    expect(round2Cells).toEqual(["Alicematch.shootout.scored", "2match.shootout.toCome"]);
    expect(round2).toHaveAttribute("aria-current", "step");
    // A long name cut short on a narrow sheet keeps its whole text as a tooltip.
    expect(within(round1).getByText("Alice")).toHaveAttribute("title", "Alice");
    expect(round1).not.toHaveAttribute("aria-current");
  });

  /**
   * Given both sides level after five kicks each
   * When the screen draws the kick sheet
   * Then a "Sudden death" heading opens the sixth round, and the band says sudden death too.
   */
  it("opens sudden death under its own heading", (): void => {
    const level = [1, 2, 3, 4, 5].flatMap(() => [
      kick("Home", "ShootoutGoal", "p1"),
      kick("Away", "ShootoutGoal", "p2"),
    ]);
    renderScreen(shootoutSnapshot(level));

    const sheet = screen.getByRole("table", { name: "match.shootout.sheetCaption" });
    const heading = within(sheet).getByText("match.shootout.suddenDeath");
    expect(heading.tagName).toBe("TH");
    expect(heading).toHaveAttribute("scope", "rowgroup");
    // The heading opens the sheet's second body, whose next row is round 6.
    const suddenDeathRows = within(heading.closest("tbody") as HTMLElement).getAllByRole("row");
    expect(suddenDeathRows.map((row) => row.firstElementChild?.textContent)).toEqual([
      "match.shootout.suddenDeath",
      "6",
    ]);
  });

  /**
   * Given a kick just taken
   * When the screen updates
   * Then a polite status region reads it out once: the kick, the club and the kicker.
   */
  it("reads out the last kick for a screen reader", (): void => {
    renderScreen(
      shootoutSnapshot([kick("Home", "ShootoutGoal", "p1"), kick("Away", "ShootoutMiss", "p2")]),
    );

    expect(screen.getByText("match.shootout.kickAnnounceMissed:1,Beta FC,Bob")).toHaveAttribute(
      "role",
      "status",
    );
  });

  /**
   * Given the shootout score is 1-0 after 1-1 in extra time
   * When the band is read by a screen reader
   * Then it reads the penalties score as words, and the match score after extra time.
   */
  /**
   * Given an away win a kick short (1-3 after the home side's fourth kick), after the final whistle
   * When the screen draws it
   * Then the band reads the away win, and the sheet's round 4 reads the home miss and the away
   * kick not taken, in words.
   */
  it("reads an away win a kick short, its last kick not taken", (): void => {
    renderScreen(
      shootoutSnapshot(
        [
          kick("Home", "ShootoutGoal", "p1"),
          kick("Away", "ShootoutGoal", "p2"),
          kick("Home", "ShootoutMiss", "p1"),
          kick("Away", "ShootoutGoal", "p2"),
          kick("Home", "ShootoutMiss", "p1"),
          kick("Away", "ShootoutGoal", "p2"),
          kick("Home", "ShootoutMiss", "p1"),
        ],
        { phase: "Finished" },
      ),
    );

    expect(screen.getByText("match.shootout.shootoutScore:1,3")).toBeInTheDocument();
    const sheet = screen.getByRole("table", { name: "match.shootout.sheetCaption" });
    const round4 = within(sheet).getAllByRole("row")[4];
    expect(
      within(round4)
        .getAllByRole("cell")
        .map((cell) => cell.textContent),
    ).toEqual(["Alicematch.shootout.missed", "match.shootout.notTaken"]);
  });

  it("reads the penalties score and the score after extra time", (): void => {
    renderScreen(
      shootoutSnapshot([kick("Home", "ShootoutGoal", "p1"), kick("Away", "ShootoutMiss", "p2")]),
    );

    expect(screen.getByText("match.shootout.shootoutScore:1,0")).toBeInTheDocument();
    expect(screen.getByText("match.shootout.afterExtraTime:1,1")).toBeInTheDocument();
  });

  /**
   * Given the engine refuses a step
   * When the screen's timer fires
   * Then the shootout stops with a message saying so, and Retry asks the engine again.
   */
  it("says so when a step fails, and retries on request", async (): Promise<void> => {
    mockedInvoke.mockRejectedValue(new Error("be.error.noLiveMatch"));
    renderScreen(shootoutSnapshot([]));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("match.shootout.stepFailed");
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(mockedInvoke).toHaveBeenCalledWith("step_live_match", { minutes: 1, careerToken: 0 });

    await act(async () => {
      fireEvent.click(within(alert).getByRole("button", { name: "common.retry" }));
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });

  /**
   * Given a shootout under way
   * When the player looks for the step control
   * Then it reads "Next kick", not "Step 1 minute".
   */
  it("names the step for a kick", (): void => {
    renderScreen(shootoutSnapshot([]));
    expect(screen.getByRole("button", { name: "match.shootout.nextKick" })).toBeInTheDocument();
  });

  /**
   * Given a paused shootout and a kick the engine has not answered yet
   * When the player presses Next kick again
   * Then no second kick is asked for: one kick at a time is the point of the screen.
   */
  it("asks for one kick at a time, however fast Next kick is pressed", async (): Promise<void> => {
    mockedInvoke.mockImplementation(() => new Promise(() => {}));
    renderScreen(shootoutSnapshot([]));
    fireEvent.click(screen.getByRole("button", { name: "match.pause" }));

    const nextKick = screen.getByRole("button", { name: "match.shootout.nextKick" });
    await act(async () => {
      fireEvent.click(nextKick);
      fireEvent.click(nextKick);
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(
      mockedInvoke.mock.calls.filter(([command]) => command === "step_live_match"),
    ).toHaveLength(1);
  });

  /**
   * Given the engine's step ends the shootout
   * When the step's snapshot arrives
   * Then the screen hands over to full time once, and steps no further.
   */
  it("hands over to full time when the last kick is taken", async (): Promise<void> => {
    const onFullTime = vi.fn();
    mockedInvoke.mockImplementation(async (command: string) =>
      command === "step_live_match"
        ? [{ minute: 121, phase: "Finished", events: [], is_finished: true }]
        : shootoutSnapshot([], { phase: "Finished" }),
    );
    renderScreen(shootoutSnapshot([]), { onFullTime });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800 + 800 + 2000);
    });
    expect(onFullTime).toHaveBeenCalledTimes(1);
    expect(
      mockedInvoke.mock.calls.filter(([command]) => command === "step_live_match"),
    ).toHaveLength(1);
    // Nothing is left to play: no Next kick, and no speed to choose.
    expect(screen.queryByRole("button", { name: "match.shootout.nextKick" })).toBeNull();
    expect(screen.queryByRole("radiogroup", { name: "match.simSpeed" })).toBeNull();
  });

  /**
   * Given a kick the engine took, but whose snapshot could not be read
   * When the player presses Retry
   * Then the snapshot is read again and no second kick is taken, and the message goes once the
   * read succeeds.
   */
  it("reads a taken kick again on Retry instead of kicking again", async (): Promise<void> => {
    let snapshotReads = 0;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "step_live_match") {
        return [{ minute: 121, phase: "PenaltyShootout", events: [], is_finished: false }];
      }
      snapshotReads += 1;
      if (snapshotReads === 1) throw new Error("be.error.noLiveMatch");
      return shootoutSnapshot([]);
    });
    renderScreen(shootoutSnapshot([]));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    const alert = screen.getByRole("alert");
    await act(async () => {
      fireEvent.click(within(alert).getByRole("button", { name: "common.retry" }));
      await vi.advanceTimersByTimeAsync(0);
    });

    const calls = (name: string) => mockedInvoke.mock.calls.filter(([command]) => command === name);
    expect(calls("step_live_match")).toHaveLength(1);
    expect(calls("get_match_snapshot")).toHaveLength(2);
    expect(screen.queryByRole("alert")).toBeNull();
  });

  /**
   * Given a step that failed and paused the shootout
   * When the player picks a speed
   * Then the message goes and the shootout plays on at that speed.
   */
  it("plays on, without the message, when a speed is chosen after a failed step", async (): Promise<void> => {
    let steps = 0;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "step_live_match") {
        steps += 1;
        if (steps === 1) throw new Error("be.error.noLiveMatch");
        return [{ minute: 121, phase: "PenaltyShootout", events: [], is_finished: false }];
      }
      return new Promise(() => {});
    });
    renderScreen(shootoutSnapshot([]));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(screen.getByRole("alert")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("radio", { name: "match.fast" }));
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(screen.queryByRole("alert")).toBeNull();
    expect(steps).toBe(2);
  });
});
