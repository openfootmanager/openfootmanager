import { useEffect, useState } from "react";

import {
  getRenewalStatusClassName,
  getRenewalStatusMessage,
  shouldDisableRenewalSubmit,
  type RenewalStatus,
} from "../playerProfile/PlayerProfile.renewal";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import {
  getFreeAgentOpeningOffer,
  offerFreeAgentContract,
  previewFreeAgentContractImpact,
  type FreeAgentContractProjection,
  type FreeAgentContractResponseData,
} from "../../services/freeAgentService";
import { resolveBackendError } from "../../utils/backendI18n";

interface UseFreeAgentContractFlowArgs {
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
}

const MAX_CONTRACT_YEARS = 5;

interface UseFreeAgentContractFlowResult {
  freeAgentTarget: PlayerData | null;
  contractWage: string;
  setContractWage: (value: string) => void;
  contractLength: string;
  setContractLength: (value: string) => void;
  contractStatus: RenewalStatus;
  contractError: string | null;
  contractFeedback: FreeAgentContractResponseData["feedback"];
  contractProjection: FreeAgentContractProjection | null;
  contractSubmitting: boolean;
  contractSubmitDisabled: boolean;
  contractStatusMessage: (t: TranslateFn) => string | null;
  contractStatusClassName: string;
  myTeam: TeamData | null;
  openFreeAgentContract: (player: PlayerData) => void;
  closeFreeAgentContract: () => void;
  submitFreeAgentContract: () => Promise<void>;
}

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

export function useFreeAgentContractFlow({
  gameState,
  onGameUpdate,
}: UseFreeAgentContractFlowArgs): UseFreeAgentContractFlowResult {
  const myTeam = gameState.teams.find((team) => team.id === gameState.manager.team_id) ?? null;
  // The id, not the player: read from the game state each render (see `useTransferBidFlow`).
  const [freeAgentTargetId, setFreeAgentTargetId] = useState<string | null>(null);
  const freeAgentTarget =
    freeAgentTargetId === null
      ? null
      : (gameState.players.find((player) => player.id === freeAgentTargetId) ?? null);
  const [contractWage, setContractWage] = useState("");
  const [contractLength, setContractLength] = useState("");
  const [contractStatus, setContractStatus] = useState<RenewalStatus>("idle");
  const [contractError, setContractError] = useState<string | null>(null);
  const [contractFeedback, setContractFeedback] =
    useState<FreeAgentContractResponseData["feedback"]>(null);
  const [contractProjection, setContractProjection] = useState<FreeAgentContractProjection | null>(
    null,
  );
  const [contractSubmitting, setContractSubmitting] = useState(false);
  const [contractSessionStatus, setContractSessionStatus] =
    useState<FreeAgentContractResponseData["session_status"]>("idle");
  const [contractSuggestedWage, setContractSuggestedWage] = useState<number | null>(null);
  const [contractSuggestedYears, setContractSuggestedYears] = useState<number | null>(null);
  const [contractIsTerminal, setContractIsTerminal] = useState(false);

  const offeredWage = Number(contractWage);
  const offeredYears = Number(contractLength);
  const isContractWageValid = Number.isFinite(offeredWage) && offeredWage > 0;
  const isContractLengthValid =
    Number.isInteger(offeredYears) && offeredYears > 0 && offeredYears <= MAX_CONTRACT_YEARS;
  const contractViolatesSoftCap =
    isContractWageValid && contractProjection !== null && !contractProjection.policy_allows;

  useEffect(() => {
    if (!freeAgentTarget || !isContractWageValid) {
      setContractProjection(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await previewFreeAgentContractImpact(freeAgentTarget.id, offeredWage);

        if (!cancelled) {
          setContractProjection(result.projection ?? null);
        }
      } catch {
        if (!cancelled) {
          setContractProjection(null);
        }
      }
    };

    void loadProjection();

    return () => {
      cancelled = true;
    };
  }, [freeAgentTarget, isContractWageValid, offeredWage]);

  useEffect(() => {
    if (freeAgentTargetId === null) {
      return;
    }

    let cancelled = false;

    const loadOpeningOffer = async (): Promise<void> => {
      try {
        const offer = await getFreeAgentOpeningOffer(freeAgentTargetId);

        if (!cancelled) {
          setContractWage(String(offer.weekly_wage));
          setContractLength(String(offer.contract_years));
        }
      } catch (error) {
        if (!cancelled) {
          setContractStatus("error");
          setContractError(resolveBackendError(error));
        }
      }
    };

    void loadOpeningOffer();

    return () => {
      cancelled = true;
    };
  }, [freeAgentTargetId]);

  const openFreeAgentContract = (player: PlayerData): void => {
    setFreeAgentTargetId(player.id);
    setContractWage("");
    setContractLength("");
    setContractStatus("idle");
    setContractError(null);
    setContractFeedback(null);
    setContractProjection(null);
    setContractSessionStatus("idle");
    setContractSuggestedWage(null);
    setContractSuggestedYears(null);
    setContractIsTerminal(false);
  };

  const closeFreeAgentContract = (): void => {
    if (contractSubmitting) {
      return;
    }

    setFreeAgentTargetId(null);
    setContractWage("");
    setContractLength("");
    setContractStatus("idle");
    setContractError(null);
    setContractFeedback(null);
    setContractProjection(null);
    setContractSessionStatus("idle");
    setContractSuggestedWage(null);
    setContractSuggestedYears(null);
    setContractIsTerminal(false);
  };

  const submitFreeAgentContract = async (): Promise<void> => {
    if (!freeAgentTarget || !isContractWageValid || !isContractLengthValid) {
      return;
    }

    setContractSubmitting(true);
    setContractStatus("idle");
    setContractError(null);

    try {
      const result = await offerFreeAgentContract(freeAgentTarget.id, offeredWage, offeredYears);

      onGameUpdate?.(result.game);
      setContractStatus(result.outcome);
      setContractFeedback(result.feedback ?? null);
      setContractSessionStatus(result.session_status);
      setContractSuggestedWage(result.suggested_wage);
      setContractSuggestedYears(result.suggested_years);
      setContractIsTerminal(result.is_terminal);

      if (result.outcome === "counter_offer") {
        if (result.suggested_wage !== null) {
          setContractWage(String(result.suggested_wage));
        }
        if (result.suggested_years !== null) {
          setContractLength(String(result.suggested_years));
        }
      }
    } catch (error) {
      setContractStatus("error");
      setContractError(resolveBackendError(error));
      setContractFeedback(null);
    } finally {
      setContractSubmitting(false);
    }
  };

  return {
    freeAgentTarget,
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    contractStatus,
    contractError,
    contractFeedback,
    contractProjection,
    contractSubmitting,
    contractSubmitDisabled: shouldDisableRenewalSubmit({
      renewalSubmitting: contractSubmitting,
      renewalIsTerminal: contractIsTerminal,
      isRenewalWageValid: isContractWageValid,
      isRenewalLengthValid: isContractLengthValid,
      renewalViolatesSoftCap: contractViolatesSoftCap,
    }),
    contractStatusMessage: (t: TranslateFn) =>
      getRenewalStatusMessage(
        {
          renewalSessionStatus: contractSessionStatus,
          renewalStatus: contractStatus,
          renewalSuggestedWage: contractSuggestedWage,
          renewalSuggestedYears: contractSuggestedYears,
          renewalError: contractError,
        },
        t,
      ),
    contractStatusClassName: getRenewalStatusClassName(contractStatus),
    myTeam,
    openFreeAgentContract,
    closeFreeAgentContract,
    submitFreeAgentContract,
  };
}
