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
  /** For actions taken elsewhere — the profile's action menu reports here too. */
  setContractActionError: (message: string | null) => void;
  terminationPreview: ContractTerminationPreviewData | null;
  showTerminationModal: boolean;
  handleMarkLetExpire: () => Promise<void>;
  handleClearLetExpire: () => Promise<void>;
  openTerminationModal: () => Promise<void>;
  closeTerminationModal: () => void;
  handleTerminateContract: () => Promise<void>;
}

/**
 * The contract decisions a manager can take on a player they own: letting the
 * deal run down, reopening it, and releasing the player outright.
 *
 * All four share a single in-flight flag. It is what disables the buttons, so
 * one decision cannot be started on top of another.
 */
export function useContractActionsFlow({
  player,
  onGameUpdate,
}: UseContractActionsFlowArgs): UseContractActionsFlowResult {
  const { t } = useTranslation();

  const [contractActionSubmitting, setContractActionSubmitting] =
    useState(false);
  const [contractActionError, setContractActionError] = useState<string | null>(
    null,
  );
  const [terminationPreview, setTerminationPreview] =
    useState<ContractTerminationPreviewData | null>(null);
  const [showTerminationModal, setShowTerminationModal] = useState(false);

  /**
   * The shape all four actions share: refuse to start if one is already
   * running, clear the last failure, then translate whatever goes wrong.
   */
  async function runContractAction(
    action: () => Promise<void>,
  ): Promise<void> {
    if (contractActionSubmitting) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);

    try {
      await action();
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  async function handleMarkLetExpire(): Promise<void> {
    await runContractAction(async () => {
      const result = await setContractExitIntent(
        player.id,
        "manager_profile_action",
      );
      onGameUpdate?.(result.game);
    });
  }

  async function handleClearLetExpire(): Promise<void> {
    await runContractAction(async () => {
      const result = await clearContractExitIntent(player.id);
      onGameUpdate?.(result.game);
    });
  }

  async function openTerminationModal(): Promise<void> {
    // Checked here as well as inside `runContractAction`, because the modal
    // must not open at all when another action holds the flag.
    if (contractActionSubmitting) {
      return;
    }

    setTerminationPreview(null);
    setShowTerminationModal(true);

    await runContractAction(async () => {
      const result = await previewContractTermination(player.id);
      setTerminationPreview(result.preview);
    });
  }

  function closeTerminationModal(): void {
    setShowTerminationModal(false);
    setTerminationPreview(null);
  }

  async function handleTerminateContract(): Promise<void> {
    // Releasing a player is only offered once its cost and squad impact are
    // known, so there is nothing to confirm without a preview.
    if (!terminationPreview) {
      return;
    }

    await runContractAction(async () => {
      const result = await terminateContractNow(player.id);
      onGameUpdate?.(result.game);
      setShowTerminationModal(false);
      setTerminationPreview(null);
    });
  }

  return {
    contractActionSubmitting,
    contractActionError,
    setContractActionError,
    terminationPreview,
    showTerminationModal,
    handleMarkLetExpire,
    handleClearLetExpire,
    openTerminationModal,
    closeTerminationModal,
    handleTerminateContract,
  };
}
