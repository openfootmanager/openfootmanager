import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import { resolveBackendText } from "../../utils/backendI18n";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  type DelegatedRenewalCaseData,
  type DelegatedRenewalResponseData,
  type NegotiationFeedbackData,
  getRenewalStatusClassName,
  getRenewalStatusMessage,
  type RenewalProjectionData,
  type RenewalResponseData,
  type RenewalStatus,
  shouldDisableRenewalSubmit,
} from "./PlayerProfile.renewal";

interface UseContractRenewalFlowArgs {
  player: PlayerData;
  gameState: GameStateData;
  /** Whether the manager's club employs an assistant who could take the talks. */
  hasAssistantManager: boolean;
  onGameUpdate?: (game: GameStateData) => void;
}

interface UseContractRenewalFlowResult {
  showRenewalModal: boolean;
  renewalWage: string;
  setRenewalWage: (value: string) => void;
  renewalLength: string;
  setRenewalLength: (value: string) => void;
  renewalSubmitting: boolean;
  renewalIsTerminal: boolean;
  renewalCooledOff: boolean;
  renewalFeedback: NegotiationFeedbackData | null;
  renewalProjection: RenewalProjectionData["projection"] | null;
  isRenewalWageValid: boolean;
  renewalViolatesSoftCap: boolean;
  renewalSubmitDisabled: boolean;
  renewalStatusMessage: string | null;
  renewalStatusClassName: string;
  openRenewalModal: () => void;
  closeRenewalModal: () => void;
  handleRenewalSubmit: () => Promise<void>;
  handleDelegateRenewal: () => Promise<void>;
}

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
 * The contract-renewal negotiation: opening the modal, the running offer, the
 * live financial projection, submitting by hand, and handing the talks to the
 * assistant manager.
 */
export function useContractRenewalFlow({
  player,
  gameState,
  hasAssistantManager,
  onGameUpdate,
}: UseContractRenewalFlowArgs): UseContractRenewalFlowResult {
  const { t } = useTranslation();

  const [showRenewalModal, setShowRenewalModal] = useState(false);
  const [renewalWage, setRenewalWage] = useState("");
  const [renewalLength, setRenewalLength] = useState("2");
  const [renewalSubmitting, setRenewalSubmitting] = useState(false);
  const [renewalStatus, setRenewalStatus] = useState<RenewalStatus>("idle");
  const [renewalError, setRenewalError] = useState<string | null>(null);
  const [renewalSuggestedWage, setRenewalSuggestedWage] = useState<
    number | null
  >(null);
  const [renewalSuggestedYears, setRenewalSuggestedYears] = useState<
    number | null
  >(null);
  const [renewalSessionStatus, setRenewalSessionStatus] =
    useState<RenewalResponseData["session_status"]>("idle");
  const [renewalIsTerminal, setRenewalIsTerminal] = useState(false);
  const [renewalCooledOff, setRenewalCooledOff] = useState(false);
  const [renewalFeedback, setRenewalFeedback] =
    useState<NegotiationFeedbackData | null>(null);
  const [renewalProjection, setRenewalProjection] = useState<
    RenewalProjectionData["projection"] | null
  >(null);

  const renewalOfferedWage = Number(renewalWage);
  const renewalOfferedYears = Number(renewalLength);
  const isRenewalWageValid =
    Number.isFinite(renewalOfferedWage) && renewalOfferedWage > 0;
  const isRenewalLengthValid =
    Number.isInteger(renewalOfferedYears) && renewalOfferedYears > 0;
  const renewalViolatesSoftCap =
    isRenewalWageValid &&
    renewalProjection !== null &&
    !renewalProjection.policy_allows;
  const renewalSubmitDisabled = shouldDisableRenewalSubmit({
    renewalSubmitting,
    renewalIsTerminal,
    isRenewalWageValid,
    isRenewalLengthValid,
    renewalViolatesSoftCap,
  });
  const renewalStatusMessage = getRenewalStatusMessage(
    {
      renewalSessionStatus,
      renewalStatus,
      renewalSuggestedWage,
      renewalSuggestedYears,
      renewalError,
    },
    t,
  );
  const renewalStatusClassName = getRenewalStatusClassName(renewalStatus);

  function openRenewalModal(): void {
    setRenewalWage(String(player.wage));
    setRenewalLength("2");
    setRenewalSubmitting(false);
    setRenewalStatus("idle");
    setRenewalError(null);
    setRenewalSuggestedWage(null);
    setRenewalSuggestedYears(null);
    setRenewalSessionStatus("idle");
    setRenewalIsTerminal(false);
    setRenewalCooledOff(false);
    setRenewalFeedback(null);
    setRenewalProjection(null);

    const renewalState = player.morale_core?.renewal_state;
    const blockedUntil = renewalState?.manager_blocked_until;
    const hasActiveManagerBlock =
      renewalState?.status === "Blocked" &&
      (!blockedUntil ||
        blockedUntil.slice(0, 10) >= gameState.clock.current_date.slice(0, 10));
    if (hasActiveManagerBlock) {
      setRenewalSessionStatus("blocked");
      setRenewalIsTerminal(true);
    }

    setShowRenewalModal(true);
  }

  function closeRenewalModal(): void {
    if (renewalSubmitting) {
      return;
    }

    setShowRenewalModal(false);
  }

  useEffect(() => {
    if (!showRenewalModal || !isRenewalWageValid) {
      setRenewalProjection(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await invoke<RenewalProjectionData>(
          "preview_renewal_financial_impact",
          {
            playerId: player.id,
            weeklyWage: renewalOfferedWage,
          },
        );

        if (!cancelled) {
          setRenewalProjection(result.projection ?? null);
        }
      } catch {
        if (!cancelled) {
          setRenewalProjection(null);
        }
      }
    };

    void loadProjection();

    return () => {
      cancelled = true;
    };
  }, [isRenewalWageValid, player.id, renewalOfferedWage, showRenewalModal]);

  async function handleRenewalSubmit(): Promise<void> {
    if (renewalSubmitDisabled) {
      return;
    }

    setRenewalSubmitting(true);
    setRenewalStatus("idle");
    setRenewalError(null);
    setRenewalCooledOff(false);

    try {
      const result = await invoke<RenewalResponseData>("propose_renewal", {
        playerId: player.id,
        weeklyWage: renewalOfferedWage,
        contractYears: renewalOfferedYears,
      });

      onGameUpdate?.(result.game);
      setRenewalStatus(result.outcome);
      setRenewalSuggestedWage(result.suggested_wage);
      setRenewalSuggestedYears(result.suggested_years);
      setRenewalSessionStatus(result.session_status);
      setRenewalIsTerminal(result.is_terminal);
      setRenewalCooledOff(result.cooled_off ?? false);
      setRenewalFeedback(result.feedback ?? null);

      if (result.session_status === "blocked") {
        setRenewalStatus("blocked");
      }

      if (result.outcome === "counter_offer") {
        if (result.suggested_wage !== null) {
          setRenewalWage(String(result.suggested_wage));
        }

        if (result.suggested_years !== null) {
          setRenewalLength(String(result.suggested_years));
        }
      }
    } catch (error) {
      setRenewalStatus("error");
      setRenewalError(resolveTranslatedErrorMessage(error, t));
      setRenewalCooledOff(false);
    } finally {
      setRenewalSubmitting(false);
    }
  }

  function applyDelegatedOutcome(
    delegatedCase: DelegatedRenewalCaseData,
  ): void {
    // `failed` is also the fallback, matching the `else` this map replaced: a
    // status the frontend does not recognise closes the talks rather than
    // leaving the modal in a half-open state.
    const outcome =
      DELEGATED_RENEWAL_OUTCOMES[delegatedCase.status] ??
      DELEGATED_RENEWAL_OUTCOMES.failed;

    setRenewalStatus(outcome.status);
    setRenewalSessionStatus(outcome.sessionStatus);
    setRenewalIsTerminal(outcome.isTerminal);
    setRenewalCooledOff(false);
    setRenewalFeedback(null);

    if (outcome.clearsSuggestedTerms) {
      setRenewalSuggestedWage(null);
      setRenewalSuggestedYears(null);
    }

    if (outcome.showsAssistantNote) {
      setRenewalError(
        resolveBackendText(
          delegatedCase.note_key,
          delegatedCase.note,
          delegatedCase.note_params,
        ),
      );
    }
  }

  async function handleDelegateRenewal(): Promise<void> {
    if (renewalSubmitting) {
      return;
    }

    if (!hasAssistantManager) {
      setRenewalStatus("error");
      setRenewalError(
        resolveTranslatedErrorMessage(
          "be.error.contracts.noAssistantManagerAssigned",
          t,
        ),
      );
      setRenewalCooledOff(false);
      return;
    }

    setRenewalSubmitting(true);
    setRenewalError(null);
    setRenewalCooledOff(false);

    try {
      const result = await invoke<DelegatedRenewalResponseData>(
        "delegate_renewals",
        {
          playerIds: [player.id],
          maxWageIncreasePct: 35,
          maxContractYears: 3,
        },
      );

      onGameUpdate?.(result.game);
      const delegatedCase: DelegatedRenewalCaseData | undefined =
        result.report.cases.find(
          (renewalCase) => renewalCase.player_id === player.id,
        );

      if (!delegatedCase) {
        setRenewalStatus("error");
        setRenewalError(t("playerProfile.renewalDelegateMissingReport"));
        return;
      }

      applyDelegatedOutcome(delegatedCase);
    } catch (error) {
      setRenewalStatus("error");
      setRenewalError(resolveTranslatedErrorMessage(error, t));
      setRenewalCooledOff(false);
    } finally {
      setRenewalSubmitting(false);
    }
  }

  return {
    showRenewalModal,
    renewalWage,
    setRenewalWage,
    renewalLength,
    setRenewalLength,
    renewalSubmitting,
    renewalIsTerminal,
    renewalCooledOff,
    renewalFeedback,
    renewalProjection,
    isRenewalWageValid,
    renewalViolatesSoftCap,
    renewalSubmitDisabled,
    renewalStatusMessage,
    renewalStatusClassName,
    openRenewalModal,
    closeRenewalModal,
    handleRenewalSubmit,
    handleDelegateRenewal,
  };
}
