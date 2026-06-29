import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  clearContractExitIntent,
  previewContractTermination,
  setContractExitIntent,
  terminateContractNow,
  type ContractTerminationPreviewData,
} from "../../services/contractService";

interface UseContractActionsFlowArgs {
  player: PlayerData;
  onGameUpdate?: (game: GameStateData) => void;
}

interface UseContractActionsFlowResult {
  contractActionSubmitting: boolean;
  contractActionError: string | null;
  terminationPreview: ContractTerminationPreviewData | null;
  showTerminationModal: boolean;
  handleMarkLetExpire: () => Promise<void>;
  handleClearLetExpire: () => Promise<void>;
  openTerminationModal: () => Promise<void>;
  handleTerminateContract: () => Promise<void>;
  closeTerminationModal: () => void;
}

export function useContractActionsFlow({
  player,
  onGameUpdate,
}: UseContractActionsFlowArgs): UseContractActionsFlowResult {
  const { t } = useTranslation();
  const [contractActionSubmitting, setContractActionSubmitting] =
    useState(false);
  const [contractActionError, setContractActionError] = useState<
    string | null
  >(null);
  const [terminationPreview, setTerminationPreview] =
    useState<ContractTerminationPreviewData | null>(null);
  const [showTerminationModal, setShowTerminationModal] = useState(false);

  async function handleMarkLetExpire(): Promise<void> {
    if (contractActionSubmitting) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);

    try {
      const result = await setContractExitIntent(
        player.id,
        "manager_profile_action",
      );
      onGameUpdate?.(result.game);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  async function handleClearLetExpire(): Promise<void> {
    if (contractActionSubmitting) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);

    try {
      const result = await clearContractExitIntent(player.id);
      onGameUpdate?.(result.game);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  async function openTerminationModal(): Promise<void> {
    if (contractActionSubmitting) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);
    setTerminationPreview(null);
    setShowTerminationModal(true);

    try {
      const result = await previewContractTermination(player.id);
      setTerminationPreview(result.preview);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  async function handleTerminateContract(): Promise<void> {
    if (contractActionSubmitting || !terminationPreview) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);

    try {
      const result = await terminateContractNow(player.id);
      onGameUpdate?.(result.game);
      setShowTerminationModal(false);
      setTerminationPreview(null);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  function closeTerminationModal(): void {
    setShowTerminationModal(false);
    setTerminationPreview(null);
  }

  return {
    contractActionSubmitting,
    contractActionError,
    terminationPreview,
    showTerminationModal,
    handleMarkLetExpire,
    handleClearLetExpire,
    openTerminationModal,
    handleTerminateContract,
    closeTerminationModal,
  };
}
