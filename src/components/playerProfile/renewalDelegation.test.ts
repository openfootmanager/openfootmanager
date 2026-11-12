import { describe, expect, it, vi } from "vitest";

import type { DelegatedRenewalCaseData } from "./PlayerProfile.renewal";
import { delegatedNegotiationPatch } from "./renewalDelegation";

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendText: (key: string | undefined, fallback: string) =>
    key ? `resolved:${key}` : fallback,
}));

const reported = (
  status: string,
  patch: Partial<DelegatedRenewalCaseData> = {},
): DelegatedRenewalCaseData => ({
  player_id: "p1",
  status: status as DelegatedRenewalCaseData["status"],
  note: "The assistant's note",
  ...patch,
});

describe("delegatedNegotiationPatch", () => {
  /**
   * Given the assistant reports a successful case
   * When it is turned into a change of the talks
   * Then the deal is agreed and over, counter-offer terms are cleared, and its note is not carried
   */
  it("agrees a successful case, clears the terms and carries no note", () => {
    const patch = delegatedNegotiationPatch(reported("successful"));

    expect(patch).toEqual({
      status: "accepted",
      sessionStatus: "agreed",
      isTerminal: true,
      cooledOff: false,
      feedback: null,
      suggestedWage: null,
      suggestedYears: null,
    });
  });

  /**
   * Given a stalled case with a note key
   * When it is turned into a change of the talks
   * Then the talks stay open, the terms are left alone, and the note is the resolved text
   */
  it("keeps a stalled case open and carries its resolved note", () => {
    const patch = delegatedNegotiationPatch(reported("stalled", { note_key: "assistant.stalled" }));

    expect(patch).toMatchObject({
      status: "rejected",
      sessionStatus: "stalled",
      isTerminal: false,
      error: "resolved:assistant.stalled",
    });
    expect(patch).not.toHaveProperty("suggestedWage");
  });

  /**
   * Given a failed case, and one whose status the screen does not know, neither with a note key
   * When each is turned into a change of the talks
   * Then both close the talks as blocked and carry the plain note
   */
  it("closes a failed or unrecognised case as blocked with the plain note", () => {
    for (const status of ["failed", "something-new"]) {
      expect(delegatedNegotiationPatch(reported(status))).toMatchObject({
        status: "blocked",
        sessionStatus: "blocked",
        isTerminal: true,
        error: "The assistant's note",
      });
    }
  });
});
