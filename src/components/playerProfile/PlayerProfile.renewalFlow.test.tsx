import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { InvokeArgs } from "@tauri-apps/api/core";
import PlayerProfile from "./PlayerProfile";
import { i18nReady } from "../../i18n";
import {
  renderOnTab,
  hasWeeklyWage,
  createPlayer,
  createStaff,
  createGameState,
  defaultInvokeResponse,
  RenewalHarness,
  type MockInvokePayload,
} from "./PlayerProfile.harness";
import { riskLevels, knowledge } from "./PlayerProfile.mocks";

/**
 * The contract renewal talks and the other contract actions on the Contract tab: validating and submitting an offer, the assistant's delegation, expiring, reopening, action failures.
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

  it("validates renewal offers before submission", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string, payload?: InvokeArgs) => {
      const args = payload as MockInvokePayload | undefined;
      if (command === "preview_renewal_financial_impact") {
        const offered = Number(args?.weeklyWage ?? 0);
        return {
          projection: {
            current_annual_wage_bill: 24000,
            projected_annual_wage_bill: 24000 - 12000 + offered,
            annual_wage_budget: 50000,
            annual_soft_cap: 55000,
            current_weekly_wage_spend: 24000,
            projected_weekly_wage_spend: 24000 - 12000 + offered,
            current_cash_runway_weeks: 1084,
            projected_cash_runway_weeks: 500,
            currently_over_budget: false,
            policy_allows: offered <= 55000,
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));

    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "0" },
    });

    expect(screen.getByText("Enter a valid weekly wage")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "60000" },
    });

    await waitFor(() => {
      expect(screen.getByText("Exceeds wage budget")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Submit Offer" })).toBeDisabled();
      expect(screen.getByText("Projected financial impact")).toBeInTheDocument();
    });
  });

  it("submits a renewal offer and refreshes contract data when accepted", async () => {
    const updatedPlayer = createPlayer({
      contract_end: "2029-08-01",
      wage: 15000,
    });
    const updatedGame = createGameState(updatedPlayer);

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "accepted",
          game: updatedGame,
          suggested_wage: null,
          suggested_years: null,
          session_status: "agreed",
          is_terminal: true,
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "15000" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("propose_renewal", {
        careerToken: 0,
        playerId: "player-1",
        weeklyWage: 15000,
        contractYears: 3,
      });
    });

    await waitFor(() => {
      expect(screen.getByText("Offer accepted")).toBeInTheDocument();
      expect(screen.getByText(/until .*1 August 2029/)).toBeInTheDocument();
      expect(
        screen.getAllByText((_, element) => hasWeeklyWage(element?.textContent ?? "", "15,000"))
          .length,
      ).toBeGreaterThan(0);
      expect(screen.getAllByText("Stable").length).toBeGreaterThan(0);
      expect(screen.getByRole("button", { name: "Done" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Submit Offer" })).not.toBeInTheDocument();
    });
  });

  it("shows a rejected state when the renewal offer is turned down", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "rejected",
          game: createGameState(createPlayer()),
          suggested_wage: null,
          suggested_years: null,
          session_status: "stalled",
          is_terminal: false,
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "12000" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(screen.getByText("Offer rejected")).toBeInTheDocument();
    });
  });

  it("shows improved terms when the player wants more", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "counter_offer",
          game: createGameState(createPlayer()),
          suggested_wage: 16000,
          suggested_years: 4,
          session_status: "open",
          is_terminal: false,
          feedback: {
            mood: "firm",
            headline_key: "playerProfile.renewalFeedbackFirmHeadline",
            detail_key: "playerProfile.renewalFeedbackFirmDetail",
            tension: 58,
            patience: 64,
            round: 1,
            params: {},
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "13000" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(screen.getByText("Wants more: €16,000/wk for 4 years")).toBeInTheDocument();
      expect(screen.getByText("Negotiation pulse")).toBeInTheDocument();
      expect(screen.getByText("They want stronger terms before moving.")).toBeInTheDocument();
      expect(screen.getByText("Round 1")).toBeInTheDocument();
      expect(screen.getByText("Patience")).toBeInTheDocument();
      expect(screen.getByText("Tension")).toBeInTheDocument();
    });
  });

  it("shows a cooled-off notice when stale talks reset before a new offer", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "counter_offer",
          game: createGameState(createPlayer()),
          suggested_wage: 15500,
          suggested_years: 3,
          session_status: "open",
          is_terminal: false,
          cooled_off: true,
          feedback: {
            mood: "calm",
            headline_key: "playerProfile.renewalFeedbackCalmHeadline",
            detail_key: "playerProfile.renewalFeedbackCalmDetail",
            tension: 34,
            patience: 76,
            round: 1,
            params: {},
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "13500" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(
        screen.getByText("Previous talks cooled off, so this starts as a fresh conversation."),
      ).toBeInTheDocument();
      expect(screen.getByText("Round 1")).toBeInTheDocument();
    });
  });

  it("can delegate a single renewal attempt to the assistant", async () => {
    const delegatedPlayer = createPlayer({
      contract_end: "2029-08-01",
      wage: 14000,
    });
    const updatedGame = createGameState(delegatedPlayer);

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "delegate_renewals") {
        return {
          game: updatedGame,
          report: {
            success_count: 1,
            failure_count: 0,
            stalled_count: 0,
            cases: [
              {
                player_id: "player-1",
                player_name: "John Smith",
                status: "successful",
                agreed_wage: 14000,
                agreed_years: 3,
                note: "I was able to close this one without needing you to step in.",
              },
            ],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={createPlayer()}
        gameState={createGameState(createPlayer(), [createStaff()])}
        isOwnClub
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.click(screen.getByRole("button", { name: "Delegate to Assistant" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("delegate_renewals", {
        careerToken: 0,
        playerIds: ["player-1"],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
    });

    await waitFor(() => {
      expect(screen.getByText("Offer accepted")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Done" })).toBeInTheDocument();
    });
  });

  it("shows a localized error when the assistant report omits the player", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "delegate_renewals") {
        return {
          game: createGameState(createPlayer()),
          report: {
            success_count: 0,
            failure_count: 0,
            stalled_count: 0,
            cases: [],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={createPlayer()}
        gameState={createGameState(createPlayer(), [createStaff()])}
        isOwnClub
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.click(screen.getByRole("button", { name: "Delegate to Assistant" }));

    await waitFor(() => {
      expect(screen.getByText("Assistant report did not include this player.")).toBeInTheDocument();
    });
  });

  // The delegated-renewal report has three outcomes and each maps to a
  // different terminal-ness: only `stalled` leaves the negotiation open for the
  // manager to take over by hand. Covered here because the two non-successful
  // branches had no test at all.
  it("leaves renewal talks open when the assistant reports a stalled case", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "delegate_renewals") {
        return {
          game: createGameState(createPlayer()),
          report: {
            success_count: 0,
            failure_count: 0,
            stalled_count: 1,
            cases: [
              {
                player_id: "player-1",
                player_name: "John Smith",
                status: "stalled",
                note: "They want to see how the season goes before signing.",
              },
            ],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={createPlayer()}
        gameState={createGameState(createPlayer(), [createStaff()])}
        isOwnClub
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.click(screen.getByRole("button", { name: "Delegate to Assistant" }));

    await waitFor(() => {
      expect(screen.getByText("Offer rejected")).toBeInTheDocument();
    });

    // Not terminal: the manager can still put an offer in themselves.
    expect(screen.getByRole("button", { name: "Submit Offer" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Done" })).toBeNull();
  });

  it("closes renewal talks when the assistant reports a failed case", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "delegate_renewals") {
        return {
          game: createGameState(createPlayer()),
          report: {
            success_count: 0,
            failure_count: 1,
            stalled_count: 0,
            cases: [
              {
                player_id: "player-1",
                player_name: "John Smith",
                status: "failed",
                note: "They have already decided to move on.",
              },
            ],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={createPlayer()}
        gameState={createGameState(createPlayer(), [createStaff()])}
        isOwnClub
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.click(screen.getByRole("button", { name: "Delegate to Assistant" }));

    await waitFor(() => {
      expect(screen.getByText("Talks are blocked after your earlier decision")).toBeInTheDocument();
    });

    // Terminal: nothing left for the manager to offer.
    expect(screen.getByRole("button", { name: "Done" })).toBeInTheDocument();
  });

  it("disables renewal delegation when no assistant manager is assigned", async () => {
    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Delegate to Assistant" })).toBeDisabled();
    });
  });

  it("localizes the no-assistant delegation error if the command still fails", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "delegate_renewals") {
        throw "be.error.contracts.noAssistantManagerAssigned";
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={createPlayer()}
        gameState={createGameState(createPlayer(), [createStaff()])}
        isOwnClub
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.click(screen.getByRole("button", { name: "Delegate to Assistant" }));

    await waitFor(() => {
      expect(
        screen.getByText("Hire an assistant manager before delegating contract talks."),
      ).toBeInTheDocument();
    });
  });

  it("marks a contract to expire and can reopen talks", async () => {
    const markedPlayer = createPlayer({
      morale_core: {
        manager_trust: 50,
        renewal_state: {
          status: "Blocked",
          manager_blocked_until: null,
          last_attempt_date: "2026-08-01",
          last_assistant_attempt_date: null,
          last_outcome: "BlockedByManager",
          conversation_round: 0,
          exit_intent: {
            kind: "let_expire",
            set_on: "2026-08-01",
            reason: "manager_profile_action",
          },
        },
      },
    });

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "set_contract_exit_intent") {
        return { game: createGameState(markedPlayer) };
      }

      if (command === "clear_contract_exit_intent") {
        return { game: createGameState(createPlayer()) };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Let Expire" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("set_contract_exit_intent", {
        careerToken: 0,
        playerId: "player-1",
        reason: "manager_profile_action",
      });
      expect(screen.getByRole("button", { name: "Reopen Talks" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "Reopen Talks" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("clear_contract_exit_intent", {
        careerToken: 0,
        playerId: "player-1",
      });
      expect(screen.getByRole("button", { name: "Let Expire" })).toBeInTheDocument();
    });
  });

  // Regression for #193 / follow-up on PR #245.
  //
  // Backend `RenewalSessionStatus::Blocked` serializes as `"Blocked"` (PascalCase)
  // — serde's default enum representation. The PR #245 fix compared against a
  // lowercase `"blocked"` literal, so the guard was always false and the block
  // was never restored on modal reopen. Existing fixtures used lowercase too,
  // so they matched the buggy frontend and the bug slipped through. This test
  // uses the real wire shape and asserts the modal opens in terminal (blocked)
  // state — only the "Done" button, no Submit path.
  it("restores blocked state when reopening the renewal modal after being blocked", async () => {
    const blockedPlayer = createPlayer({
      morale_core: {
        manager_trust: 30,
        renewal_state: {
          status: "Blocked",
          manager_blocked_until: "2099-01-01",
          last_attempt_date: "2026-08-01",
          last_assistant_attempt_date: null,
          last_outcome: "BlockedByManager",
          conversation_round: 1,
          exit_intent: null,
        },
      },
    });

    renderOnTab("Contract tab", <RenewalHarness initialPlayer={blockedPlayer} />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Done" })).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "Submit Offer" })).not.toBeInTheDocument();
  });

  it("localizes contract action failures instead of showing raw backend keys", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "set_contract_exit_intent") {
        throw "be.error.contracts.playerHasNoActiveContract";
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Let Expire" }));

    await waitFor(() => {
      expect(screen.getByText("This player does not have an active contract.")).toBeInTheDocument();
    });
  });

  // The four contract actions share one `submitting` flag, and it is what
  // disables the buttons. Without it a second action could be started on top of
  // the first, so no termination preview may be requested while an exit intent
  // is still in flight.
  it("locks the contract actions while one of them is in flight", async () => {
    let releaseExitIntent: (() => void) | undefined;

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "set_contract_exit_intent") {
        await new Promise<void>((resolve) => {
          releaseExitIntent = resolve;
        });
        return { game: createGameState(createPlayer()) };
      }

      return defaultInvokeResponse(command);
    });

    renderOnTab("Contract tab", <RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Let Expire" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("set_contract_exit_intent", {
        careerToken: 0,
        playerId: "player-1",
        reason: "manager_profile_action",
      });
    });

    const terminate = screen.getByRole("button", { name: "Terminate Now" });
    expect(terminate).toBeDisabled();

    // The sheet loads one preview with the tab; a click on the locked button asks for no second one.
    const previewsAsked = () =>
      vi.mocked(invoke).mock.calls.filter(([command]) => command === "preview_contract_termination")
        .length;
    const askedBeforeTheClick = previewsAsked();
    fireEvent.click(terminate);
    expect(previewsAsked()).toBe(askedBeforeTheClick);

    releaseExitIntent?.();
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Terminate Now" })).toBeEnabled();
    });
  });
});
