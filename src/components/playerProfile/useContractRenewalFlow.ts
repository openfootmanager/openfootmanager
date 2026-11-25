import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import { invokeForCareer } from "../../services/careerInvoke";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  type DelegatedRenewalCaseData,
  type DelegatedRenewalResponseData,
  type NegotiationFeedbackData,
  getRenewalStatusClassName,
  getRenewalStatusMessage,
  hasActiveManagerRenewalBlock,
  type RenewalProjectionData,
  type RenewalResponseData,
  shouldDisableRenewalSubmit,
} from "./PlayerProfile.renewal";
import { delegatedNegotiationPatch } from "./renewalDelegation";
import { useRenewalNegotiation } from "./useRenewalNegotiation";
import { useRenewalProjection } from "./useRenewalProjection";

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

/**
 * The contract-renewal negotiation: opening the modal, the running offer, the
 * live financial projection, submitting by hand, and handing the talks to the
 * assistant manager.
 *
 * The talks' state is `useRenewalNegotiation`, the projection is
 * `useRenewalProjection`, and how the assistant's report changes the talks is
 * `delegatedNegotiationPatch`; this holds the offer being typed and the calls.
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
  const { negotiation, patch, reset } = useRenewalNegotiation();

  const renewalOfferedWage = Number(renewalWage);
  const renewalOfferedYears = Number(renewalLength);
  const isRenewalWageValid = Number.isFinite(renewalOfferedWage) && renewalOfferedWage > 0;
  const isRenewalLengthValid = Number.isInteger(renewalOfferedYears) && renewalOfferedYears > 0;
  const { projection: renewalProjection, clearProjection } = useRenewalProjection(
    player.id,
    renewalOfferedWage,
    showRenewalModal && isRenewalWageValid,
  );
  const renewalViolatesSoftCap =
    isRenewalWageValid && renewalProjection !== null && !renewalProjection.policy_allows;
  const renewalSubmitDisabled = shouldDisableRenewalSubmit({
    renewalSubmitting,
    renewalIsTerminal: negotiation.isTerminal,
    isRenewalWageValid,
    isRenewalLengthValid,
    renewalViolatesSoftCap,
  });
  const renewalStatusMessage = getRenewalStatusMessage(
    {
      renewalSessionStatus: negotiation.sessionStatus,
      renewalStatus: negotiation.status,
      renewalSuggestedWage: negotiation.suggestedWage,
      renewalSuggestedYears: negotiation.suggestedYears,
      renewalError: negotiation.error,
    },
    t,
  );
  const renewalStatusClassName = getRenewalStatusClassName(negotiation.status);

  function openRenewalModal(): void {
    setRenewalWage(String(player.wage));
    setRenewalLength("2");
    setRenewalSubmitting(false);
    reset();
    clearProjection();

    if (hasActiveManagerRenewalBlock(player, gameState.clock.current_date)) {
      patch({ sessionStatus: "blocked", isTerminal: true });
    }

    setShowRenewalModal(true);
  }

  function closeRenewalModal(): void {
    if (renewalSubmitting) {
      return;
    }

    setShowRenewalModal(false);
  }

  async function handleRenewalSubmit(): Promise<void> {
    if (renewalSubmitDisabled) {
      return;
    }

    setRenewalSubmitting(true);
    patch({ status: "idle", error: null, cooledOff: false });

    try {
      const result = await invokeForCareer<RenewalResponseData>("propose_renewal", {
        playerId: player.id,
        weeklyWage: renewalOfferedWage,
        contractYears: renewalOfferedYears,
      });

      onGameUpdate?.(result.game);
      patch({
        status: result.session_status === "blocked" ? "blocked" : result.outcome,
        suggestedWage: result.suggested_wage,
        suggestedYears: result.suggested_years,
        sessionStatus: result.session_status,
        isTerminal: result.is_terminal,
        cooledOff: result.cooled_off ?? false,
        feedback: result.feedback ?? null,
      });

      if (result.outcome === "counter_offer") {
        if (result.suggested_wage !== null) {
          setRenewalWage(String(result.suggested_wage));
        }

        if (result.suggested_years !== null) {
          setRenewalLength(String(result.suggested_years));
        }
      }
    } catch (error) {
      patch({ status: "error", error: resolveTranslatedErrorMessage(error), cooledOff: false });
    } finally {
      setRenewalSubmitting(false);
    }
  }

  async function handleDelegateRenewal(): Promise<void> {
    if (renewalSubmitting) {
      return;
    }

    if (!hasAssistantManager) {
      patch({
        status: "error",
        error: resolveTranslatedErrorMessage("be.error.contracts.noAssistantManagerAssigned"),
        cooledOff: false,
      });
      return;
    }

    setRenewalSubmitting(true);
    patch({ error: null, cooledOff: false });

    try {
      const result = await invokeForCareer<DelegatedRenewalResponseData>("delegate_renewals", {
        playerIds: [player.id],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });

      onGameUpdate?.(result.game);
      const delegatedCase: DelegatedRenewalCaseData | undefined = result.report.cases.find(
        (renewalCase) => renewalCase.player_id === player.id,
      );

      if (!delegatedCase) {
        patch({ status: "error", error: t("playerProfile.renewalDelegateMissingReport") });
        return;
      }

      patch(delegatedNegotiationPatch(delegatedCase));
    } catch (error) {
      patch({ status: "error", error: resolveTranslatedErrorMessage(error), cooledOff: false });
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
    renewalIsTerminal: negotiation.isTerminal,
    renewalCooledOff: negotiation.cooledOff,
    renewalFeedback: negotiation.feedback,
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
