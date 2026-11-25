import { resolveBackendText } from "../../utils/backendI18n";
import type {
  DelegatedRenewalCaseData,
  RenewalResponseData,
  RenewalStatus,
} from "./PlayerProfile.renewal";
import type { RenewalNegotiation } from "./useRenewalNegotiation";

interface DelegatedRenewalOutcome {
  status: RenewalStatus;
  sessionStatus: RenewalResponseData["session_status"];
  isTerminal: boolean;
  /**
   * Whether the assistant's own note explains this outcome. A successful case
   * needs no explanation, so its note is not surfaced.
   */
  showsAssistantNote: boolean;
  /** An agreed deal retires any counter-offer terms from an earlier round. */
  clearsSuggestedTerms: boolean;
}

/**
 * How each outcome the assistant can report maps onto the negotiation state.
 *
 * `isTerminal` is the one the player actually feels: only a stalled case leaves
 * the modal open so they can take the talks back over by hand. The other two
 * collapse it to a single Done button.
 */
const DELEGATED_RENEWAL_OUTCOMES: Record<
  DelegatedRenewalCaseData["status"],
  DelegatedRenewalOutcome
> = {
  successful: {
    status: "accepted",
    sessionStatus: "agreed",
    isTerminal: true,
    showsAssistantNote: false,
    clearsSuggestedTerms: true,
  },
  stalled: {
    status: "rejected",
    sessionStatus: "stalled",
    isTerminal: false,
    showsAssistantNote: true,
    clearsSuggestedTerms: false,
  },
  failed: {
    status: "blocked",
    sessionStatus: "blocked",
    isTerminal: true,
    showsAssistantNote: true,
    clearsSuggestedTerms: false,
  },
};

/**
 * What the assistant's report on a case changes in the talks: the outcome's status, whether they
 * are over, no cooled-off notice or feedback, the counter-offer terms cleared on an agreed deal,
 * and the assistant's own note when it explains the outcome.
 */
export function delegatedNegotiationPatch(
  delegatedCase: DelegatedRenewalCaseData,
): Partial<RenewalNegotiation> {
  // `failed` is also the fallback, matching the `else` this map replaced: a
  // status the frontend does not recognise closes the talks rather than
  // leaving the modal in a half-open state.
  const outcome =
    DELEGATED_RENEWAL_OUTCOMES[delegatedCase.status] ?? DELEGATED_RENEWAL_OUTCOMES.failed;

  return {
    status: outcome.status,
    sessionStatus: outcome.sessionStatus,
    isTerminal: outcome.isTerminal,
    cooledOff: false,
    feedback: null,
    ...(outcome.clearsSuggestedTerms ? { suggestedWage: null, suggestedYears: null } : {}),
    ...(outcome.showsAssistantNote
      ? {
          error: resolveBackendText(
            delegatedCase.note_key,
            delegatedCase.note,
            delegatedCase.note_params,
          ),
        }
      : {}),
  };
}
