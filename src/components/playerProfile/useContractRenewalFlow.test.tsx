import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { invokeForCareer } from "../../services/careerInvoke";
import { previewRenewalProjection } from "../../services/contractService";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import type { DelegatedRenewalCaseData, RenewalResponseData } from "./PlayerProfile.renewal";
import { useContractRenewalFlow } from "./useContractRenewalFlow";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, string | number>) =>
      options ? `${key} ${JSON.stringify(options)}` : key,
  }),
}));

vi.mock("../../services/careerInvoke", () => ({ invokeForCareer: vi.fn() }));
vi.mock("../../services/contractService", () => ({ previewRenewalProjection: vi.fn() }));
vi.mock("../../utils/backendI18n", () => ({
  resolveBackendText: (key: string | undefined, fallback: string) =>
    key ? `resolved:${key}` : fallback,
}));
vi.mock("../../utils/errorMessage", () => ({
  resolveTranslatedErrorMessage: (error: unknown) =>
    `err:${error instanceof Error ? error.message : String(error)}`,
}));

const invoke = vi.mocked(invokeForCareer);
const projectionFor = vi.mocked(previewRenewalProjection);

const TODAY = "2026-08-01T00:00:00Z";

const gameState = { clock: { current_date: TODAY } } as GameStateData;

function aPlayer(patch: Partial<PlayerData> = {}): PlayerData {
  return { id: "p1", wage: 12_000, ...patch } as PlayerData;
}

function projection(policyAllows = true) {
  return { projection: { policy_allows: policyAllows } } as Awaited<
    ReturnType<typeof previewRenewalProjection>
  >;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function response(patch: Partial<RenewalResponseData> = {}): RenewalResponseData {
  return {
    outcome: "accepted",
    game: { marker: "updated" } as unknown as GameStateData,
    suggested_wage: null,
    suggested_years: null,
    session_status: "agreed",
    is_terminal: true,
    ...patch,
  };
}

function delegated(status: string, patch: Partial<DelegatedRenewalCaseData> = {}) {
  return {
    game: { marker: "delegated" } as unknown as GameStateData,
    report: {
      success_count: 0,
      failure_count: 0,
      stalled_count: 0,
      cases: [
        {
          player_id: "p1",
          status: status as DelegatedRenewalCaseData["status"],
          note: "The assistant's note",
          ...patch,
        },
      ],
    },
  };
}

function setup(
  options: { player?: PlayerData; hasAssistantManager?: boolean; onGameUpdate?: () => void } = {},
) {
  const onGameUpdate = options.onGameUpdate ?? vi.fn();
  const hook = renderHook(() =>
    useContractRenewalFlow({
      player: options.player ?? aPlayer(),
      gameState,
      hasAssistantManager: options.hasAssistantManager ?? true,
      onGameUpdate,
    }),
  );
  return { ...hook, onGameUpdate };
}

function open(result: { current: ReturnType<typeof useContractRenewalFlow> }) {
  act(() => result.current.openRenewalModal());
}

beforeEach(() => {
  vi.clearAllMocks();
  projectionFor.mockResolvedValue(projection());
});

describe("useContractRenewalFlow: opening", () => {
  /**
   * Given a hook that has not been opened
   * When it is read
   * Then the modal is shut, the offer is empty with a two-year length, and submitting is off
   */
  it("starts shut with an empty offer", () => {
    const { result } = setup();

    expect(result.current.showRenewalModal).toBe(false);
    expect(result.current.renewalWage).toBe("");
    expect(result.current.renewalLength).toBe("2");
    expect(result.current.renewalSubmitDisabled).toBe(true);
  });

  /**
   * Given a player on 12,000 a week
   * When the modal is opened
   * Then it is open with his wage as the offer, two years, nothing said, and submitting is on
   */
  it("opens with the player's wage as the offer", () => {
    const { result } = setup();

    open(result);

    expect(result.current.showRenewalModal).toBe(true);
    expect(result.current.renewalWage).toBe("12000");
    expect(result.current.renewalLength).toBe("2");
    expect(result.current.renewalStatusMessage).toBeNull();
    expect(result.current.renewalIsTerminal).toBe(false);
    expect(result.current.renewalSubmitDisabled).toBe(false);
  });

  /**
   * Given a player the manager is blocked from renewing until after today
   * When the modal is opened
   * Then the talks are already over: terminal, said as blocked, submitting off
   */
  it("opens already blocked while the manager's block is active", () => {
    const player = aPlayer({
      morale_core: {
        renewal_state: { status: "Blocked", manager_blocked_until: "2026-09-01" },
      },
    } as unknown as Partial<PlayerData>);
    const { result } = setup({ player });

    open(result);

    expect(result.current.renewalIsTerminal).toBe(true);
    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalBlocked");
    expect(result.current.renewalSubmitDisabled).toBe(true);
  });

  const blockedUntil = (until?: string) =>
    aPlayer({
      morale_core: {
        renewal_state: { status: "Blocked", manager_blocked_until: until },
      },
    } as unknown as Partial<PlayerData>);

  /**
   * Given a block that ended before today
   * When the modal is opened
   * Then the talks are open: the block is over
   */
  it("opens for talks when the manager's block ended before today", () => {
    const { result } = setup({ player: blockedUntil("2026-07-01") });

    open(result);

    expect(result.current.renewalIsTerminal).toBe(false);
  });

  /**
   * Given a block with no end date
   * When the modal is opened
   * Then the talks are over: an undated block is still active
   */
  it("opens blocked when the manager's block has no end date", () => {
    const { result } = setup({ player: blockedUntil(undefined) });

    open(result);

    expect(result.current.renewalIsTerminal).toBe(true);
  });

  /**
   * Given a block that ends today (the clock reads 1 August with a time of day)
   * When the modal is opened
   * Then it is still blocked: the block holds through the day it names
   */
  it("keeps the talks blocked through the day the block ends", () => {
    const player = aPlayer({
      morale_core: {
        renewal_state: { status: "Blocked", manager_blocked_until: "2026-08-01" },
      },
    } as unknown as Partial<PlayerData>);
    const { result } = setup({ player });

    open(result);

    expect(result.current.renewalIsTerminal).toBe(true);
    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalBlocked");
  });

  async function submitOffer(result: { current: ReturnType<typeof useContractRenewalFlow> }) {
    await act(async () => {
      await result.current.handleRenewalSubmit();
    });
  }

  /**
   * Given talks the player accepted (over, said as accepted, on a 15,000 offer for 3 years)
   * When the modal is closed and opened again
   * Then the offer is back to his wage for two years, nothing is said, and the talks are not over
   */
  it("starts afresh when reopened after an accepted offer", async () => {
    invoke.mockResolvedValue(response());
    const { result } = setup();
    open(result);
    act(() => {
      result.current.setRenewalWage("15000");
      result.current.setRenewalLength("3");
    });
    await submitOffer(result);
    expect(result.current.renewalIsTerminal).toBe(true);
    act(() => result.current.closeRenewalModal());

    open(result);

    expect(result.current.renewalWage).toBe("12000");
    expect(result.current.renewalLength).toBe("2");
    expect(result.current.renewalStatusMessage).toBeNull();
    expect(result.current.renewalIsTerminal).toBe(false);
    expect(result.current.renewalSubmitDisabled).toBe(false);
  });

  /**
   * Given talks in which the player countered with 18,000 for 4 years (the offer filled with them)
   * When the modal is closed and opened again
   * Then the offer is back to his wage for two years and the counter is no longer said
   */
  it("starts afresh when reopened after a counter-offer", async () => {
    invoke.mockResolvedValue(
      response({
        outcome: "counter_offer",
        suggested_wage: 18_000,
        suggested_years: 4,
        session_status: "open",
        is_terminal: false,
      }),
    );
    const { result } = setup();
    open(result);
    await submitOffer(result);
    expect(result.current.renewalWage).toBe("18000");
    expect(result.current.renewalStatusMessage).toContain("playerProfile.renewalCounter");
    act(() => result.current.closeRenewalModal());

    open(result);

    expect(result.current.renewalWage).toBe("12000");
    expect(result.current.renewalLength).toBe("2");
    expect(result.current.renewalStatusMessage).toBeNull();
  });

  /**
   * Given talks that carry a cooled-off notice and negotiation feedback
   * When the modal is closed and opened again
   * Then neither is held any more
   */
  it("starts afresh when reopened after a cooled-off notice and feedback", async () => {
    const feedback = { summary: "Close" } as unknown as RenewalResponseData["feedback"];
    invoke.mockResolvedValue(
      response({
        outcome: "rejected",
        session_status: "open",
        is_terminal: false,
        cooled_off: true,
        feedback,
      }),
    );
    const { result } = setup();
    open(result);
    await submitOffer(result);
    expect(result.current.renewalCooledOff).toBe(true);
    expect(result.current.renewalFeedback).toBe(feedback);
    act(() => result.current.closeRenewalModal());

    open(result);

    expect(result.current.renewalCooledOff).toBe(false);
    expect(result.current.renewalFeedback).toBeNull();
    expect(result.current.renewalStatusMessage).toBeNull();
  });

  /**
   * Given an open modal
   * When it is closed
   * Then it is shut; and while an offer is in flight, closing does nothing
   */
  it("closes, but not while an offer is in flight", async () => {
    const pending = deferred<RenewalResponseData>();
    invoke.mockReturnValue(pending.promise);
    const { result } = setup();
    open(result);

    act(() => result.current.closeRenewalModal());
    expect(result.current.showRenewalModal).toBe(false);

    open(result);
    act(() => {
      void result.current.handleRenewalSubmit();
    });
    await waitFor(() => expect(result.current.renewalSubmitting).toBe(true));
    act(() => result.current.closeRenewalModal());
    expect(result.current.showRenewalModal).toBe(true);

    await act(async () => pending.resolve(response()));
    expect(result.current.renewalSubmitting).toBe(false);
  });
});

describe("useContractRenewalFlow: the financial projection", () => {
  /**
   * Given an open modal with a valid wage
   * When the projection is asked for
   * Then it is asked for that player and wage, and the answer is held
   */
  it("loads the projection for the offered wage", async () => {
    const { result } = setup();

    open(result);

    await waitFor(() => expect(result.current.renewalProjection).not.toBeNull());
    expect(projectionFor).toHaveBeenCalledWith("p1", 12_000);
  });

  /**
   * Given a modal that has not been opened
   * When it is read
   * Then nothing has been asked for
   */
  it("asks for nothing while the modal is shut", () => {
    setup();

    expect(projectionFor).not.toHaveBeenCalled();
  });

  /**
   * Given an open modal whose valid wage has already been asked about (counted after it opens)
   * When the wage is changed to "0", which is not valid
   * Then nothing more is asked for, and there is no projection
   */
  it("asks for nothing more once the wage is not a valid number", () => {
    const { result } = setup();
    open(result);
    const callsAfterOpening = projectionFor.mock.calls.length;
    expect(callsAfterOpening).toBeGreaterThan(0);

    act(() => result.current.setRenewalWage("0"));

    expect(result.current.isRenewalWageValid).toBe(false);
    expect(projectionFor.mock.calls.length).toBe(callsAfterOpening);
    expect(result.current.renewalProjection).toBeNull();
  });

  /**
   * Given a projection the policy refuses (over the wage budget) for 12,000, held
   * When the wage is changed to 20,000 and that request fails
   * Then the earlier projection is dropped: none is held, and the soft cap no longer restricts
   *      submitting (the catch clears what an answer for another wage left)
   */
  it("drops the earlier projection when the request for the next wage fails", async () => {
    projectionFor.mockImplementation((_id, wage) =>
      wage === 12_000 ? Promise.resolve(projection(false)) : Promise.reject(new Error("offline")),
    );
    const { result } = setup();
    open(result);
    await waitFor(() => expect(result.current.renewalViolatesSoftCap).toBe(true));

    act(() => result.current.setRenewalWage("20000"));

    await waitFor(() => expect(result.current.renewalProjection).toBeNull());
    expect(projectionFor).toHaveBeenCalledWith("p1", 20_000);
    expect(result.current.renewalViolatesSoftCap).toBe(false);
    expect(result.current.renewalSubmitDisabled).toBe(false);
  });

  /**
   * Given a projection request that fails
   * When it settles
   * Then there is no projection and nothing throws
   */
  it("holds no projection when the request fails", async () => {
    projectionFor.mockRejectedValue(new Error("offline"));
    const { result } = setup();

    open(result);

    await waitFor(() => expect(projectionFor).toHaveBeenCalled());
    expect(result.current.renewalProjection).toBeNull();
  });

  /**
   * Given a slow answer for one wage and a quick answer for the wage typed after it
   * When the slow one arrives last
   * Then the quick one stays: an answer for an earlier wage is dropped
   */
  it("drops a projection for a wage that was changed before it arrived", async () => {
    const slow = deferred<ReturnType<typeof projection>>();
    projectionFor.mockImplementation((_id, wage) =>
      wage === 12_000 ? slow.promise : Promise.resolve(projection(false)),
    );
    const { result } = setup();
    open(result);
    act(() => result.current.setRenewalWage("20000"));
    await waitFor(() => expect(result.current.renewalProjection?.policy_allows).toBe(false));

    await act(async () => slow.resolve(projection(true)));

    expect(result.current.renewalProjection?.policy_allows).toBe(false);
  });

  /**
   * Given a projection still on its way
   * When the modal is closed before it arrives
   * Then it is dropped and none is held
   */
  it("drops a projection that arrives after the modal was closed", async () => {
    const late = deferred<ReturnType<typeof projection>>();
    projectionFor.mockReturnValue(late.promise);
    const { result } = setup();
    open(result);
    act(() => result.current.closeRenewalModal());

    await act(async () => late.resolve(projection()));

    expect(result.current.renewalProjection).toBeNull();
  });

  /**
   * Given a projection the policy refuses (over the wage budget)
   * When the wage is valid
   * Then the offer is said to break the soft cap and submitting is off
   */
  it("turns submitting off when the projection breaks the soft cap", async () => {
    projectionFor.mockResolvedValue(projection(false));
    const { result } = setup();

    open(result);

    await waitFor(() => expect(result.current.renewalViolatesSoftCap).toBe(true));
    expect(result.current.renewalSubmitDisabled).toBe(true);
  });
});

describe("useContractRenewalFlow: submitting an offer", () => {
  async function submit(result: { current: ReturnType<typeof useContractRenewalFlow> }) {
    await act(async () => {
      await result.current.handleRenewalSubmit();
    });
  }

  /**
   * Given an open modal with an offer of 15,000 for 3 years
   * When it is submitted and the player accepts
   * Then the proposal goes out with those terms, the new game is handed up, the talks are over
   *      and said as accepted
   */
  it("sends the offer and takes the accepted answer", async () => {
    invoke.mockResolvedValue(response());
    const { result, onGameUpdate } = setup();
    open(result);
    act(() => {
      result.current.setRenewalWage("15000");
      result.current.setRenewalLength("3");
    });

    await submit(result);

    expect(invoke).toHaveBeenCalledWith("propose_renewal", {
      playerId: "p1",
      weeklyWage: 15_000,
      contractYears: 3,
    });
    expect(onGameUpdate).toHaveBeenCalledWith({ marker: "updated" });
    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalAccepted");
    expect(result.current.renewalStatusClassName).toBe("text-positive");
    expect(result.current.renewalIsTerminal).toBe(true);
    expect(result.current.renewalSubmitting).toBe(false);
  });

  /**
   * Given an offer the player turns down
   * When it is submitted
   * Then it is said as rejected, in the danger colour, and the talks stay open
   */
  it("says a rejected offer as rejected", async () => {
    invoke.mockResolvedValue(
      response({ outcome: "rejected", session_status: "open", is_terminal: false }),
    );
    const { result } = setup();
    open(result);

    await submit(result);

    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalRejected");
    expect(result.current.renewalStatusClassName).toBe("text-danger");
    expect(result.current.renewalIsTerminal).toBe(false);
  });

  /**
   * Given a counter-offer of 18,000 for 4 years
   * When the offer is submitted
   * Then the wage and the length are filled with the counter's terms, and it is said as a counter
   *      in the warning colour
   */
  it("fills the offer with a counter-offer's wage and years", async () => {
    invoke.mockResolvedValue(
      response({
        outcome: "counter_offer",
        suggested_wage: 18_000,
        suggested_years: 4,
        session_status: "open",
        is_terminal: false,
      }),
    );
    const { result } = setup();
    open(result);

    await submit(result);

    expect(result.current.renewalWage).toBe("18000");
    expect(result.current.renewalLength).toBe("4");
    expect(result.current.renewalStatusMessage).toContain("playerProfile.renewalCounter");
    expect(result.current.renewalStatusMessage).toContain('"years":4');
    expect(result.current.renewalStatusClassName).toBe("text-warning");
  });

  /**
   * Given a counter-offer that names a wage and no length
   * When the offer is submitted
   * Then the wage is filled and the length is left as it was
   */
  it("leaves the length alone when a counter-offer names none", async () => {
    invoke.mockResolvedValue(
      response({
        outcome: "counter_offer",
        suggested_wage: 18_000,
        suggested_years: null,
        session_status: "open",
        is_terminal: false,
      }),
    );
    const { result } = setup();
    open(result);
    act(() => result.current.setRenewalLength("3"));

    await submit(result);

    expect(result.current.renewalWage).toBe("18000");
    expect(result.current.renewalLength).toBe("3");
  });

  /**
   * Given an answer whose session is blocked
   * When the offer is submitted
   * Then it is said as blocked and the talks are over
   */
  it("says a blocked session as blocked", async () => {
    invoke.mockResolvedValue(
      response({ outcome: "rejected", session_status: "blocked", is_terminal: true }),
    );
    const { result } = setup();
    open(result);

    await submit(result);

    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalBlocked");
    expect(result.current.renewalIsTerminal).toBe(true);
  });

  /**
   * Given an answer that carries a cooled-off notice and negotiation feedback
   * When the offer is submitted, and then submitted again
   * Then both are held after the first, and the notice is cleared as the second goes out
   */
  it("holds the cooled-off notice and the feedback, and clears the notice on the next offer", async () => {
    const feedback = { summary: "Close" } as unknown as RenewalResponseData["feedback"];
    invoke.mockResolvedValueOnce(
      response({
        outcome: "rejected",
        session_status: "open",
        is_terminal: false,
        cooled_off: true,
        feedback,
      }),
    );
    const { result } = setup();
    open(result);
    await submit(result);
    expect(result.current.renewalCooledOff).toBe(true);
    expect(result.current.renewalFeedback).toBe(feedback);

    const second = deferred<RenewalResponseData>();
    invoke.mockReturnValueOnce(second.promise);
    act(() => {
      void result.current.handleRenewalSubmit();
    });
    await waitFor(() => expect(result.current.renewalCooledOff).toBe(false));
    await act(async () => second.resolve(response()));
  });

  /**
   * Given a proposal that fails
   * When it is submitted
   * Then the backend's error is said in the danger colour, no notice is held, and submitting is
   *      free again
   */
  it("says a failed proposal as an error", async () => {
    invoke.mockRejectedValue(new Error("be.error.boom"));
    const { result } = setup();
    open(result);

    await submit(result);

    expect(result.current.renewalStatusMessage).toBe("err:be.error.boom");
    expect(result.current.renewalStatusClassName).toBe("text-danger");
    expect(result.current.renewalCooledOff).toBe(false);
    expect(result.current.renewalSubmitting).toBe(false);
  });

  /**
   * Given a wage that is not valid
   * When an offer is submitted
   * Then nothing is sent
   */
  it("sends nothing while submitting is off", async () => {
    const { result } = setup();
    open(result);
    act(() => result.current.setRenewalWage("0"));

    await submit(result);

    expect(invoke).not.toHaveBeenCalled();
  });
});

describe("useContractRenewalFlow: handing the talks to the assistant", () => {
  async function delegate(result: { current: ReturnType<typeof useContractRenewalFlow> }) {
    await act(async () => {
      await result.current.handleDelegateRenewal();
    });
  }

  /**
   * Given a club with no assistant manager
   * When the talks are delegated
   * Then nothing is sent and the no-assistant error is said
   */
  it("refuses without an assistant manager", async () => {
    const { result } = setup({ hasAssistantManager: false });
    open(result);

    await delegate(result);

    expect(invoke).not.toHaveBeenCalled();
    expect(result.current.renewalStatusMessage).toBe(
      "err:be.error.contracts.noAssistantManagerAssigned",
    );
    expect(result.current.renewalStatusClassName).toBe("text-danger");
  });

  /**
   * Given a delegation in flight
   * When the talks are delegated again
   * Then the second does nothing
   */
  it("ignores a second delegation while one is in flight", async () => {
    const pending = deferred<ReturnType<typeof delegated>>();
    invoke.mockReturnValue(pending.promise as never);
    const { result } = setup();
    open(result);
    act(() => {
      void result.current.handleDelegateRenewal();
    });
    await waitFor(() => expect(result.current.renewalSubmitting).toBe(true));

    await delegate(result);

    expect(invoke).toHaveBeenCalledTimes(1);
    await act(async () => pending.resolve(delegated("successful")));
  });

  /**
   * Given a successful case
   * When the talks are delegated
   * Then the assistant is asked with its limits for this player, the new game is handed up, the
   *      talks are over and said as accepted, and its note is not shown
   */
  it("takes a successful delegation as accepted", async () => {
    invoke.mockResolvedValue(delegated("successful") as never);
    const { result, onGameUpdate } = setup();
    open(result);

    await delegate(result);

    expect(invoke).toHaveBeenCalledWith("delegate_renewals", {
      playerIds: ["p1"],
      maxWageIncreasePct: 35,
      maxContractYears: 3,
    });
    expect(onGameUpdate).toHaveBeenCalledWith({ marker: "delegated" });
    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalAccepted");
    expect(result.current.renewalIsTerminal).toBe(true);
    expect(result.current.renewalSubmitting).toBe(false);
  });

  /**
   * Given a stalled case
   * When the talks are delegated
   * Then they stay open for the manager to take back over, and are said as rejected
   */
  it("leaves the talks open after a stalled delegation", async () => {
    invoke.mockResolvedValue(delegated("stalled") as never);
    const { result } = setup();
    open(result);

    await delegate(result);

    expect(result.current.renewalIsTerminal).toBe(false);
    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalRejected");
    expect(result.current.renewalStatusClassName).toBe("text-danger");
  });

  /**
   * Given a case the assistant reports as failed, and a case with a status the screen does not know
   * When the talks are delegated
   * Then the talks close, said as blocked, for each
   */
  it.each(["failed", "something-new"])(
    "closes the talks after a delegation reported as %s",
    async (status) => {
      invoke.mockResolvedValue(delegated(status) as never);
      const { result } = setup();
      open(result);

      await delegate(result);

      expect(result.current.renewalIsTerminal).toBe(true);
      expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalBlocked");
    },
  );

  /**
   * Given a report that has no case for this player
   * When the talks are delegated
   * Then the new game is still handed up, and the missing-report message is said as an error
   */
  it("says a report without this player as an error", async () => {
    const answer = delegated("successful");
    answer.report.cases[0].player_id = "someone-else";
    invoke.mockResolvedValue(answer as never);
    const { result, onGameUpdate } = setup();
    open(result);

    await delegate(result);

    expect(onGameUpdate).toHaveBeenCalledWith({ marker: "delegated" });
    expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalDelegateMissingReport");
    expect(result.current.renewalStatusClassName).toBe("text-danger");
    expect(result.current.renewalSubmitting).toBe(false);
  });

  /**
   * Given a delegation that fails
   * When the talks are delegated
   * Then the backend's error is said and submitting is free again
   */
  it("says a failed delegation as an error", async () => {
    invoke.mockRejectedValue(new Error("be.error.boom"));
    const { result } = setup();
    open(result);

    await delegate(result);

    expect(result.current.renewalStatusMessage).toBe("err:be.error.boom");
    expect(result.current.renewalSubmitting).toBe(false);
  });
});
