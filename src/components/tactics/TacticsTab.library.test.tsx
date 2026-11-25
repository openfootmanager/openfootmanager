import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import type { CustomTactic } from "../../services/customTacticsService";
import TacticsTab from "./TacticsTab";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import { makeGameState, makeTeam } from "../../test-utils/tacticsTabFixtures";

// Whether the backend's position table has arrived; a test says it has not.
const positionTableState = vi.hoisted(() => ({ arrived: true }));

vi.mock("../../hooks/useFamiliarity", async () => {
  const { cannedFamiliarityModule } = await import("../../test-utils/familiarity");
  const { familiarityState } = await import("../../test-utils/tacticsTabFixtures");
  return cannedFamiliarityModule(
    (playerId) => familiarityState.players.get(playerId),
    () => familiarityState.arrived,
  );
});

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

vi.mock("react-i18next", () => ({
  // The error banner pulls in utils/backendI18n -> i18n/index, which registers this plugin.
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) => {
      if (key === "playerProfile.daysRemaining") {
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")} days remaining`;
      }
      if (key === "playerProfile.injuryDaysShort") {
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")}d`;
      }
      if (key.startsWith("common.injuries.")) {
        return String((fallback as Record<string, unknown> | undefined)?.defaultValue ?? key);
      }
      return typeof fallback === "string" ? fallback : key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("TacticsTab tactic library", () => {
  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    const defaultGameState = makeGameState();
    const defaultRoster = defaultGameState.players.filter((p) => p.team_id === "team1");
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return defaultRoster;
      return defaultGameState;
    });
  });

  it("can duplicate the current setup into a custom tactic shell", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "tactics.tacticActions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.duplicateTactic" }));

    expect(screen.getByRole("button", { name: "tactics.updateTactic" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    expect(screen.getByRole("option", { name: /tactics.copyOfTactic/i })).toBeInTheDocument();
  });

  /** Makes the faked backend keep saved tactics, as the save does, on top of its other answers. */
  function keepSavedTactics() {
    const saved: CustomTactic[] = [];
    const others = mockedInvoke.getMockImplementation();
    mockedInvoke.mockImplementation(async (command: string, args?: InvokeArgs) => {
      if (command === "list_custom_tactics") return structuredClone(saved);
      if (command === "save_custom_tactic" || command === "save_current_tactic") {
        const tactic = (args as { tactic: CustomTactic }).tactic;
        const at = saved.findIndex((existing) => existing.id === tactic.id);
        if (at === -1) saved.push(tactic);
        else saved[at] = tactic;
        return structuredClone(saved);
      }
      return others ? others(command, args) : undefined;
    });
    return saved;
  }

  /**
   * Given a custom tactic made and saved
   * When the screen is closed and opened again
   * Then the tactic is still in the library: it comes from the save, not from the screen.
   */
  it("keeps custom tactics across a save and reload", async () => {
    keepSavedTactics();
    const gameState = makeGameState();
    const { unmount } = render(
      <TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "tactics.tacticActions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.duplicateTactic" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith(
        "save_current_tactic",
        expect.objectContaining({ tactic: expect.objectContaining({ formation: "4-4-2" }) }),
      ),
    );
    unmount();

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));

    expect(
      await screen.findByRole("option", { name: /tactics.copyOfTactic/i }),
    ).toBeInTheDocument();
  });

  /**
   * Given a custom tactic made while managing one club
   * When the same manager takes charge of another club
   * Then the tactic is still in the library: the library belongs to the career, not the club.
   */
  it("keeps custom tactics when the manager changes club", async () => {
    keepSavedTactics();
    const originalState = makeGameState();
    const otherClub = makeGameState();
    otherClub.manager.team_id = "team2";
    otherClub.teams = [makeTeam({ id: "team2", manager_id: otherClub.manager.id })];
    otherClub.players = otherClub.players.map((player) => ({ ...player, team_id: "team2" }));

    const { unmount } = render(
      <TacticsTab gameState={originalState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "tactics.tacticActions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.duplicateTactic" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("save_current_tactic", expect.anything()),
    );
    unmount();

    render(<TacticsTab gameState={otherClub} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));

    expect(
      await screen.findByRole("option", { name: /tactics.copyOfTactic/i }),
    ).toBeInTheDocument();
  }, 15000);

  /**
   * Given a library an older version kept in the browser
   * When the screen opens, and opens again
   * Then its tactics are saved once, the old key is gone, and the second opening adds nothing.
   */
  it("copies an old browser library into the save once", async () => {
    const saved = keepSavedTactics();
    const gameState = makeGameState();
    localStorage.setItem(
      `ofm:tactics:custom:${gameState.manager.id}:${gameState.clock.start_date}:team1`,
      JSON.stringify([
        {
          id: "custom:old",
          type: "custom",
          name: "Old favourite",
          description: "",
          formation: "4-4-2",
          playStyle: "Balanced",
          sourcePresetName: null,
        },
      ]),
    );

    const first = render(
      <TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    expect(await screen.findByRole("option", { name: /Old favourite/ })).toBeInTheDocument();
    first.unmount();
    const secondRender = render(
      <TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    expect(await screen.findByRole("option", { name: /Old favourite/ })).toBeInTheDocument();
    secondRender.unmount();

    expect(saved.map((tactic) => tactic.id)).toEqual(["custom:old"]);
    // Copied, never moved: the old key is shared by careers begun the same year.
    expect(localStorage.length).toBe(1);
    const saves = mockedInvoke.mock.calls.filter(([command]) => command === "save_custom_tactic");
    expect(saves).toHaveLength(1);
  });

  it("does not mark a preset as active when applying it fails", async () => {
    const gameState = makeGameState();
    mockedInvoke.mockImplementation(async (command) => {
      if (command === "apply_tactic_preset") {
        throw new Error("boom");
      }

      if (command === "get_squad") {
        return gameState.players.filter((p) => p.team_id === "team1");
      }

      return gameState;
    });

    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    expect(screen.getByRole("button", { name: "tactics.chooseTactic" })).toHaveTextContent(
      "balanced-control",
    );

    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    fireEvent.click(screen.getByRole("option", { name: /high-press/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        careerToken: 0,
        formation: "3-4-3",
        keepInstructions: true,
        playStyle: "HighPress",
      });
    });

    expect(screen.getByRole("button", { name: "tactics.chooseTactic" })).toHaveTextContent(
      "balanced-control",
    );
  });
});
