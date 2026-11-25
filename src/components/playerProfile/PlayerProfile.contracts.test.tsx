import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import PlayerProfile from "./PlayerProfile";
import { i18nReady } from "../../i18n";
import {
  renderOnTab,
  createPlayer,
  createStaff,
  createGameState,
  defaultInvokeResponse,
  RenewalHarness,
} from "./PlayerProfile.harness";
import { riskLevels, knowledge } from "./PlayerProfile.mocks";

/**
 * Arriving straight into a contract modal, terminating a contract, and signing a free agent.
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

  // The dashboard opens a profile straight into one of these modals when the
  // player arrives from an inbox action. Both are driven by effects, so nothing
  // is clicked and no other test in this file reaches them.
  it("opens the renewal modal on arrival when asked to", async () => {
    const player = createPlayer();

    render(
      <PlayerProfile
        player={player}
        gameState={createGameState(player, [createStaff()])}
        isOwnClub
        startWithRenewalModal
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Submit Offer" })).toBeInTheDocument();
    });
  });

  it("opens the termination modal on arrival when asked to", async () => {
    const player = createPlayer();

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "preview_contract_termination") {
        return {
          preview: {
            player_id: "player-1",
            player_name: "J. Smith",
            severance_cost: 132000,
            squad_safety: {
              team_id: "team-1",
              projected_roster_size: 11,
              healthy_players: 11,
              healthy_goalkeepers: 1,
              effective_xi_size: 11,
              can_field_matchday_squad: true,
              missing_reasons: [],
            },
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    render(
      <PlayerProfile
        player={player}
        gameState={createGameState(player, [createStaff()])}
        isOwnClub
        startWithTerminationModal
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("preview_contract_termination", {
        playerId: "player-1",
      });
    });
    await waitFor(() => {
      expect(screen.getByText("Severance")).toBeInTheDocument();
    });
  });

  // `isOwnClub={false}` alone is not enough: the manager still owns the
  // contract whenever the player's club is theirs. Only a player at another
  // club is genuinely not theirs to renew or release.
  it("leaves both modals shut for a player at another club", async () => {
    const player = createPlayer({ team_id: "team-2" });

    render(
      <PlayerProfile
        player={player}
        gameState={createGameState(player, [createStaff()])}
        isOwnClub={false}
        startWithRenewalModal
        startWithTerminationModal
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("playerProfile.contractSheet.his")).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "Submit Offer" })).toBeNull();
    expect(invoke).not.toHaveBeenCalledWith("preview_contract_termination", expect.anything());
  });

  it("previews and confirms immediate contract termination", async () => {
    const releasedPlayer = createPlayer({
      team_id: null,
      contract_end: null,
      wage: 0,
    });

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "preview_contract_termination") {
        return {
          preview: {
            player_id: "player-1",
            player_name: "J. Smith",
            severance_cost: 132000,
            squad_safety: {
              team_id: "team-1",
              projected_roster_size: 11,
              healthy_players: 11,
              healthy_goalkeepers: 1,
              effective_xi_size: 11,
              can_field_matchday_squad: true,
              missing_reasons: [],
            },
          },
        };
      }

      if (command === "terminate_contract_now") {
        return {
          game: createGameState(releasedPlayer),
          severance_cost: 132000,
          squad_safety: {
            team_id: "team-1",
            projected_roster_size: 11,
            healthy_players: 11,
            healthy_goalkeepers: 1,
            effective_xi_size: 11,
            can_field_matchday_squad: true,
            missing_reasons: [],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Terminate Now" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("preview_contract_termination", {
        playerId: "player-1",
      });
      expect(screen.getByText("Severance")).toBeInTheDocument();
      expect(screen.getByText("11/11")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "Terminate Contract" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("terminate_contract_now", {
        careerToken: 0,
        playerId: "player-1",
      });
      expect(screen.getByText("No Contract")).toBeInTheDocument();
    });
  });
});

describe("PlayerProfile free agent signing", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation(async (command: string) => defaultInvokeResponse(command));
  });

  it("shows Offer Contract button for a free agent player", () => {
    const freeAgent = createPlayer({
      team_id: null,
      contract_end: null,
      wage: 0,
    });
    const gameState = createGameState(freeAgent);

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={freeAgent}
        gameState={gameState}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Offer Contract" })).toBeInTheDocument();
  });

  it("does not show Offer Contract for a retired free agent", () => {
    const retired = createPlayer({
      team_id: null,
      contract_end: null,
      wage: 0,
      retired: true,
    });
    const gameState = createGameState(retired);

    render(
      <PlayerProfile
        player={retired}
        gameState={gameState}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.queryByRole("button", { name: "Offer Contract" })).not.toBeInTheDocument();
  });

  it("opens the contract modal when Offer Contract is clicked", async () => {
    const freeAgent = createPlayer({
      team_id: null,
      contract_end: null,
      wage: 0,
      market_value: 600_000,
    });
    const gameState = createGameState(freeAgent);

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={freeAgent}
        gameState={gameState}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Offer Contract" }));

    await waitFor(() => {
      expect(screen.getByRole("dialog", { name: "Offer Contract" })).toBeInTheDocument();
    });
  });

  it("submits offer_free_agent_contract and updates game state on acceptance", async () => {
    const freeAgent = createPlayer({
      team_id: null,
      contract_end: null,
      wage: 0,
      market_value: 600_000,
    });
    const signedPlayer = createPlayer({
      team_id: "team-1",
      contract_end: "2029-08-01",
      wage: 4_000,
    });
    const onGameUpdate = vi.fn();

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_free_agent_opening_offer") {
        return { weekly_wage: 3000, contract_years: 3 };
      }
      if (command === "offer_free_agent_contract") {
        return {
          outcome: "accepted",
          game: createGameState(signedPlayer),
          suggested_wage: null,
          suggested_years: null,
          session_status: "agreed",
          is_terminal: true,
          cooled_off: false,
          feedback: null,
        };
      }
      return defaultInvokeResponse(command);
    });

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={freeAgent}
        gameState={createGameState(freeAgent)}
        isOwnClub={false}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Offer Contract" }));

    await waitFor(() => {
      expect(screen.getByRole("dialog", { name: "Offer Contract" })).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Submit Offer" })).toBeEnabled();
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("offer_free_agent_contract", {
        careerToken: 0,
        playerId: "player-1",
        weeklyWage: 3000,
        contractYears: 3,
      });
      expect(onGameUpdate).toHaveBeenCalled();
    });
  });
});
