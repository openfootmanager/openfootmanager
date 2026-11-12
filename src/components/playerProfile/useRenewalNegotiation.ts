import { useCallback, useState } from "react";

import type {
  NegotiationFeedbackData,
  RenewalResponseData,
  RenewalStatus,
} from "./PlayerProfile.renewal";

/** Where the talks stand: what was last said, what the player asked for, and whether they are over. */
export interface RenewalNegotiation {
  status: RenewalStatus;
  error: string | null;
  suggestedWage: number | null;
  suggestedYears: number | null;
  sessionStatus: RenewalResponseData["session_status"];
  isTerminal: boolean;
  cooledOff: boolean;
  feedback: NegotiationFeedbackData | null;
}

/** Talks that have not started: nothing said, nothing asked, not over. */
export const IDLE_NEGOTIATION: RenewalNegotiation = {
  status: "idle",
  error: null,
  suggestedWage: null,
  suggestedYears: null,
  sessionStatus: "idle",
  isTerminal: false,
  cooledOff: false,
  feedback: null,
};

/**
 * The state of the renewal talks, kept as one value so an answer from the backend (the player's, or
 * the assistant's) is applied as one patch instead of a setter per field. A patch changes only the
 * fields it names; `reset` puts the talks back to the start.
 */
export function useRenewalNegotiation() {
  const [negotiation, setNegotiation] = useState<RenewalNegotiation>(IDLE_NEGOTIATION);

  const patch = useCallback((changes: Partial<RenewalNegotiation>) => {
    setNegotiation((current) => ({ ...current, ...changes }));
  }, []);

  const reset = useCallback(() => {
    setNegotiation(IDLE_NEGOTIATION);
  }, []);

  return { negotiation, patch, reset };
}
