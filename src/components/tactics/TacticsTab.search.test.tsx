import { Profiler, useState } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { i18nReady } from "../../i18n";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import type { PlayerData } from "../../store/gameStore";
import {
  ControlledTacticsTab,
  fakeBackend,
  mockedInvoke,
  resetHarness,
  seedSavedTactics,
} from "./TacticsTab.harness";

/**
 * Typing in a search on the Tactics screen must leave the screen standing. Two searches can be
 * meant: the tactic picker's own, and the dashboard header's, whose every keystroke re-renders the
 * tab's parent. Real i18n, and the Tactics tab's shared harness.
 */

vi.mock("../../hooks/useFamiliarity", async () => {
  const { cannedFamiliarityModule } = await import("../../test-utils/familiarity");
  const { familiarityState } = await import("../../test-utils/tacticsFamiliarityState");
  return cannedFamiliarityModule(
    (playerId) => familiarityState.players.get(playerId) as PlayerData | undefined,
    () => familiarityState.arrived,
  );
});

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  return cannedPositionsModule();
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const TACTICS_TAB_CASE_TIMEOUT_MS = 20_000;

/**
 * More commits than typing ten characters can explain: a render loop's footprint. A loop does not
 * fail a test by itself (the run just hangs), so the Profiler below throws past this many commits.
 */
const RENDER_BUDGET = 400;

/** Counts a tree's commits, and throws once there are more than RENDER_BUDGET: a loop fails fast. */
function countCommits(counter: { commits: number }) {
  return () => {
    counter.commits += 1;
    if (counter.commits > RENDER_BUDGET) throw new Error("render loop: too many commits");
  };
}

/** A stand-in for the dashboard: a header search whose every keystroke re-renders the tab under it. */
function DashboardWithHeaderSearch({ onCommit }: { onCommit: () => void }) {
  const [query, setQuery] = useState("");
  return (
    <div>
      <input
        aria-label="Global search"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <Profiler id="tactics" onRender={onCommit}>
        <ControlledTacticsTab />
      </Profiler>
    </div>
  );
}

describe("typing in a search on the Tactics screen", {
  timeout: TACTICS_TAB_CASE_TIMEOUT_MS,
}, () => {
  let consoleError: ReturnType<typeof vi.spyOn>;

  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    resetHarness();
    seedSavedTactics([
      {
        id: "custom:saved",
        name: "Saved custom",
        description: "",
        formation: "4-4-2",
        playStyle: "Balanced",
      },
    ]);
    fakeBackend();
    consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    consoleError.mockRestore();
  });

  /** Types `text` one character at a time, as a keyboard does. */
  function typeInto(field: HTMLElement, text: string) {
    for (let length = 1; length <= text.length; length += 1) {
      fireEvent.change(field, { target: { value: text.slice(0, length) } });
    }
  }

  /**
   * Given the tactic picker open on the Tactics screen
   * When a name is typed into its search, narrowing the list to one tactic and then to none
   * Then the screen is still there, nothing was logged as an error, and the list shows what matches
   */
  it("keeps the screen when typing in the tactic picker's search", async () => {
    const counter = { commits: 0 };
    render(
      <Profiler id="tab" onRender={countCommits(counter)}>
        <ControlledTacticsTab />
      </Profiler>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    const search = await screen.findByRole("textbox", { name: /Search tactics/ });

    typeInto(search, "saved");
    expect(await screen.findByRole("option", { name: /Saved custom/ })).toBeInTheDocument();
    typeInto(search, "zzzzzz");

    await waitFor(() => expect(screen.queryByRole("option")).toBeNull());
    expect(screen.getByRole("button", { name: "Choose tactic" })).toBeInTheDocument();
    expect(consoleError).not.toHaveBeenCalled();
    expect(counter.commits).toBeLessThan(RENDER_BUDGET);
  });

  /**
   * Given the Tactics screen under a dashboard whose header search re-renders it on each keystroke
   * When a player's name is typed into the header search
   * Then the screen is still there, nothing was logged as an error, and it did not render in a loop
   */
  it("keeps the screen when typing in the dashboard header's search", async () => {
    const counter = { commits: 0 };
    render(<DashboardWithHeaderSearch onCommit={countCommits(counter)} />);
    await screen.findByRole("button", { name: "Choose tactic" });
    const before = counter.commits;

    typeInto(screen.getByRole("textbox", { name: "Global search" }), "player");
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalled());

    expect(screen.getByRole("button", { name: "Choose tactic" })).toBeInTheDocument();
    expect(consoleError).not.toHaveBeenCalled();
    expect(counter.commits - before).toBeLessThan(RENDER_BUDGET);
  });
});
