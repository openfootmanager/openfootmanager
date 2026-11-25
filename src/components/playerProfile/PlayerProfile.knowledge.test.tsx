import { render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { InvokeArgs } from "@tauri-apps/api/core";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import PlayerProfile from "./PlayerProfile";
import { i18nReady } from "../../i18n";
import {
  hasWeeklyWage,
  createPlayer,
  createStaff,
  createGameState,
  defaultInvokeResponse,
} from "./PlayerProfile.harness";
import { riskLevels, knowledge, wages } from "./PlayerProfile.mocks";

/**
 * What the profile shows of a player by how the club knows him: a scouted snapshot, a rival, a stranger, the club's own.
 * Split from the single `PlayerProfile.test.tsx` (#1406): every test keeps its name and its
 * Given/When/Then; the mocks and fixtures are in `PlayerProfile.mocks.ts` and `PlayerProfile.harness.tsx`.
 */

vi.mock("../../hooks/useContractWarningStages", async () => {
  const { contractWarningStagesMock } = await import("./PlayerProfile.mocks");
  return contractWarningStagesMock;
});
vi.mock("../../hooks/usePlayerKnowledge", async () => {
  const { playerKnowledgeMock } = await import("./PlayerProfile.mocks");
  return playerKnowledgeMock;
});
vi.mock("../../hooks/useVisibleWage", async () => {
  const { visibleWageMock } = await import("./PlayerProfile.mocks");
  return visibleWageMock;
});
vi.mock("../../hooks/usePotentialOpinion", async () => {
  const { potentialOpinionMock } = await import("./PlayerProfile.mocks");
  return potentialOpinionMock;
});
vi.mock("../../hooks/usePositionRatings", async () => {
  const { positionRatingsMock } = await import("./PlayerProfile.mocks");
  return positionRatingsMock;
});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", async () => {
  const { reactI18nextMock } = await import("./PlayerProfile.mocks");
  return reactI18nextMock;
});
vi.mock("../../utils/backendI18n", async (importOriginal) => {
  const { backendI18nOverrides } = await import("./PlayerProfile.mocks");
  return {
    ...(await importOriginal<typeof import("../../utils/backendI18n")>()),
    ...backendI18nOverrides,
  };
});
vi.mock("../../lib/countries", async () => {
  const { countriesMock } = await import("./PlayerProfile.mocks");
  return countriesMock;
});

beforeAll(async () => {
  await i18nReady;
});

describe("PlayerProfile contract surfaces", () => {
  beforeEach(() => {
    riskLevels.current = {};
    knowledge.current = {};
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation(async (command: string) => defaultInvokeResponse(command));
  });

  /**
   * Given a player at another club whom a scout reported on, flagged out of date by the backend
   * When the profile opens
   * Then the scouted attributes show instead of the hidden placeholder, marked out of date
   */
  it("shows a scouted player's snapshot attributes with the backend's out-of-date verdict", async () => {
    const player = createPlayer({ team_id: "team-2" });
    knowledge.current = {
      [player.id]: { kind: "scouted", scouted_on: "2026-03-10", current: false },
    };
    vi.mocked(invoke).mockImplementation(async (command: string, args?: InvokeArgs) => {
      if (command === "get_scouted_report") {
        expect(args).toEqual({ playerId: player.id });
        return {
          player_id: player.id,
          scouted_on: "2026-03-10",
          attributes: { ...player.attributes, pace: 77 },
          out_of_date: true,
        };
      }
      return defaultInvokeResponse(command);
    });

    render(
      <PlayerProfile
        player={player}
        gameState={createGameState(player, [createStaff()])}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(await screen.findByText("playerProfile.scoutReportOutOfDate")).toBeInTheDocument();
    expect(screen.getByText("77")).toBeInTheDocument();
    expect(screen.queryAllByText("playerProfile.attributesHidden")).toHaveLength(0);
  });

  /**
   * Given a player nobody has scouted
   * When the profile opens
   * Then the attributes stay hidden
   */
  it("keeps the attributes hidden when the backend has no report", async () => {
    const player = createPlayer({ team_id: "team-2" });
    knowledge.current = { [player.id]: { kind: "unknown" } };
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_scouted_report") return null;
      return defaultInvokeResponse(command);
    });

    render(
      <PlayerProfile
        player={player}
        gameState={createGameState(player, [createStaff()])}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_scouted_report", { playerId: player.id });
    });
    // The Overview's attributes card and the Attributes tab each say so.
    expect(screen.getAllByText("playerProfile.attributesHidden")).toHaveLength(2);
  });
});

describe("PlayerProfile shows only what the club knows", () => {
  beforeEach(() => {
    riskLevels.current = {};
    knowledge.current = {};
    knowledge.revisions = [];
    wages.current = {};
    wages.revisions = [];
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation(async (command: string) => defaultInvokeResponse(command));
  });

  // Renders a rival's profile, then the same profile on `next`, and says whether the profile handed
  // the knowledge and wage hooks a new revision for it: whether they would ask the backend again.
  function asksAgainOn(next: (gameState: GameStateData) => GameStateData) {
    const player = rival();
    const gameState = createGameState(player);
    const { rerender } = render(
      <PlayerProfile
        player={player}
        gameState={gameState}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );
    const last = (revisions: string[]) => revisions[revisions.length - 1];
    const before = [last(knowledge.revisions), last(wages.revisions)];
    rerender(
      <PlayerProfile
        player={player}
        gameState={next(gameState)}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );
    return [last(knowledge.revisions) !== before[0], last(wages.revisions) !== before[1]];
  }

  /**
   * Given a rival's profile open on the last day of a season
   * When the season rolls over on the same date (promotion and relegation change who is known)
   * Then the profile asks again what the club knows and which wage it may see
   */
  it("asks again about a rival when the season rolls over on the same date", () => {
    expect(
      asksAgainOn((gameState) => ({
        ...gameState,
        season_context: { ...gameState.season_context, season_start: "2099-07-01" },
      })),
    ).toEqual([true, true]);
  });

  /**
   * Given a rival's profile open
   * When another career is installed under it, on the same date, with the same clubs
   * Then the profile asks again what the club knows and which wage it may see
   */
  it("asks again about a rival when another career is installed under the profile", () => {
    expect(
      asksAgainOn((gameState) => ({
        ...gameState,
        career_token: (gameState.career_token ?? 0) + 1,
      })),
    ).toEqual([true, true]);
  });

  // Condition 80, morale 75, a weekly wage of 12K (also on his contract in the movement history)
  // and the Wonderkid tag: each one something a club only knows about its own player. The tag
  // shows only as the club's opinion (PlayerProfile.potential.test.tsx); his own copy, which
  // tells his true potential, never shows.
  function rival(): PlayerData {
    return createPlayer({
      team_id: "team-2",
      ovr: 77,
      condition: 80,
      morale: 75,
      traits: ["Wonderkid"],
      movement_history: [
        {
          date: "2025-07-01",
          kind: "initial_contract",
          to_team_id: "team-2",
          to_team_name: "Rival FC",
          contract: {
            start: "2025-07-01",
            end: "2028-06-30",
            weekly_wage: 12_000,
            source: "initial",
          },
        },
      ],
    });
  }

  // The movement history's wage on a contract, which only the club's own player shows.
  // The wage of the contracts he has had is a column of the Contract tab's history table, which is
  // there only when the club may see his wage (the Movement cards no longer print a contract).
  const showsMovementWage = () =>
    screen.queryByText("playerProfile.contractSheet.weeklyWage", { selector: "th" }) !== null;

  function renderProfile(player: PlayerData) {
    render(
      <PlayerProfile
        player={player}
        gameState={createGameState(player, [createStaff()])}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );
  }

  // The Overview's Details card's wage row: its value, apart from its label.
  const contractWageValue = () =>
    screen.getByText("Wage").parentElement?.textContent?.replace("Wage", "").trim();

  // A row of the Overview's Now card (Condition, Morale): its value, apart from its label.
  const nowValue = (label: string) =>
    within(screen.getByRole("region", { name: "Now" }))
      .getByText(label)
      .parentElement?.textContent?.replace(label, "")
      .trim();

  const showsWage = () =>
    screen.queryAllByText((_, element) => hasWeeklyWage(element?.textContent ?? "", "12,000"))
      .length > 0;

  /**
   * Given a player at a club in another division, never scouted
   * When his profile opens
   * Then his OVR, condition, morale, wage, attributes and Wonderkid tag are not shown; his market
   *      value, which is public, is
   */
  it("shows only public facts about a player the club does not know", async () => {
    const player = rival();
    knowledge.current = { [player.id]: { kind: "unknown" } };
    wages.current = { [player.id]: { kind: "hidden" } };

    renderProfile(player);

    expect(await screen.findAllByText("playerProfile.attributesHidden")).toHaveLength(2);
    expect(nowValue("Condition")).toBe("–");
    expect(nowValue("Morale")).toBe("–");
    expect(showsWage()).toBe(false);
    expect(contractWageValue()).toBe("–");
    expect(showsMovementWage()).toBe(false);
    expect(screen.queryByText("77")).not.toBeInTheDocument();
    expect(screen.queryByText(/Wonderkid/)).not.toBeInTheDocument();
    expect(screen.getAllByText("€350K").length).toBeGreaterThan(0);
  });

  /**
   * Given a player at a club in the manager's own division, never scouted
   * When his profile opens
   * Then his OVR and attributes show, as the staff see him every season; the Wonderkid tag, which
   *      tells his potential, waits for a scout; his condition, morale and wage stay his club's
   *      business
   */
  it("shows a known rival's attributes but not his potential or his club's private state", async () => {
    const player = rival();
    knowledge.current = { [player.id]: { kind: "known" } };
    wages.current = { [player.id]: { kind: "hidden" } };

    renderProfile(player);

    expect(await screen.findByText("77")).toBeInTheDocument();
    expect(screen.queryByText(/Wonderkid/)).not.toBeInTheDocument();
    expect(screen.queryAllByText("playerProfile.attributesHidden")).toHaveLength(0);
    expect(nowValue("Condition")).toBe("–");
    expect(nowValue("Morale")).toBe("–");
    expect(showsWage()).toBe(false);
    expect(contractWageValue()).toBe("–");
    expect(showsMovementWage()).toBe(false);
  });

  /**
   * Given a player in another division whom a scout reported on this season
   * When his profile opens
   * Then the scout's attributes show and his wage reads as about the figure, but not his OVR (no
   *      estimate yet), his condition, his morale or his own Wonderkid tag
   */
  it("shows a scouted player's report but not what the scout could not see", async () => {
    const player = rival();
    knowledge.current = {
      [player.id]: { kind: "scouted", scouted_on: "2026-09-01", current: true },
    };
    wages.current = { [player.id]: { kind: "approximate", weekly: 16_000 } };
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_scouted_report") {
        return {
          player_id: player.id,
          scouted_on: "2026-09-01",
          attributes: player.attributes,
          out_of_date: false,
        };
      }
      return defaultInvokeResponse(command);
    });

    renderProfile(player);

    await waitFor(() => {
      expect(screen.queryAllByText("playerProfile.attributesHidden")).toHaveLength(0);
    });
    expect(screen.queryByText(/Wonderkid/)).not.toBeInTheDocument();
    expect(nowValue("Condition")).toBe("–");
    expect(screen.queryByText("77")).not.toBeInTheDocument();
    expect(contractWageValue()).toBe("about €16,000/wk");
    expect(showsMovementWage()).toBe(false);
  });

  /**
   * Given the club's own player
   * When his profile opens
   * Then his condition, morale and exact wage show; his own Wonderkid tag does not, as only the
   *      club's opinion of him tags him
   */
  it("shows the club's own player's private state but not his own Wonderkid tag", () => {
    const player = createPlayer({
      ovr: 77,
      condition: 80,
      morale: 75,
      traits: ["Wonderkid"],
      movement_history: rival().movement_history,
    });
    knowledge.current = { [player.id]: { kind: "own" } };

    render(<PlayerProfile player={player} gameState={createGameState(player)} isOwnClub />);

    expect(nowValue("Condition")).toContain("80");
    expect(nowValue("Morale")).toBe("75%");
    expect(showsWage()).toBe(true);
    expect(contractWageValue()).not.toBe("–");
    expect(showsMovementWage()).toBe(true);
    expect(screen.getByText("77")).toBeInTheDocument();
    expect(screen.queryByText(/Wonderkid/)).not.toBeInTheDocument();
  });
});
