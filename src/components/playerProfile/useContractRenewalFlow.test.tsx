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

  /**
   * Given a block that ended before today, and a block with no end date
   * When the modal is opened for each
   * Then the lapsed one is open for talks and the undated one is still blocked
   */
  it("treats a lapsed block as over and an undated one as active", () => {
    const blocked = (until?: string) =>
      aPlayer({
        morale_core: {
          renewal_state: { status: "Blocked", manager_blocked_until: until },
        },
      } as unknown as Partial<PlayerData>);

    const lapsed = setup({ player: blocked("2026-07-01") });
    open(lapsed.result);
    const undated = setup({ player: blocked(undefined) });
    open(undated.result);

    expect(lapsed.result.current.renewalIsTerminal).toBe(false);
    expect(undated.result.current.renewalIsTerminal).toBe(true);
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
   * Given a shut modal, and an open one whose wage is not a valid number
   * When each is read
   * Then nothing is asked for and there is no projection
   */
  it("asks for nothing while the modal is shut or the wage is not valid", () => {
    const { result } = setup();
    expect(projectionFor).not.toHaveBeenCalled();

    open(result);
    act(() => result.current.setRenewalWage("0"));

    expect(result.current.isRenewalWageValid).toBe(false);
    expect(result.current.renewalProjection).toBeNull();
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
   * Given a failed case, and a case with a status the screen does not know
   * When the talks are delegated for each
   * Then both close the talks, said as blocked
   */
  it("closes the talks after a failed or unrecognised delegation", async () => {
    for (const status of ["failed", "something-new"]) {
      invoke.mockResolvedValue(delegated(status) as never);
      const { result } = setup();
      open(result);

      await delegate(result);

      expect(result.current.renewalIsTerminal).toBe(true);
      expect(result.current.renewalStatusMessage).toBe("playerProfile.renewalBlocked");
    }
  });

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
