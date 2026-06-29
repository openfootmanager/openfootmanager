import { formatExactMoney } from "../../lib/helpers";
import DashboardModalFrame from "../dashboard/DashboardModalFrame";
import { Button } from "../ui";
import type { ContractTerminationPreviewData } from "../../services/contractService";

type TranslateFn = (
  key: string,
  options?: Record<string, string | number>,
) => string;

interface PlayerProfileTerminationModalProps {
  playerName: string;
  terminationPreview: ContractTerminationPreviewData | null;
  contractActionError: string | null;
  contractActionSubmitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
  t: TranslateFn;
}

export default function PlayerProfileTerminationModal({
  playerName,
  terminationPreview,
  contractActionError,
  contractActionSubmitting,
  onCancel,
  onConfirm,
  t,
}: PlayerProfileTerminationModalProps) {
  return (
    <DashboardModalFrame maxWidthClassName="max-w-lg">
      <div className="space-y-4">
        <div>
          <h2 className="font-heading text-lg font-bold text-gray-900 dark:text-gray-100">
            {t("playerProfile.terminateContractTitle")}
          </h2>
          <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
            {t("playerProfile.terminateContractBody", {
              name: playerName,
            })}
          </p>
        </div>

        {terminationPreview ? (
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4 text-sm dark:border-navy-600 dark:bg-navy-700/60">
            <div className="flex items-center justify-between gap-4">
              <span className="text-gray-500 dark:text-gray-400">
                {t("playerProfile.terminationSeverance")}
              </span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {formatExactMoney(terminationPreview.severance_cost)}
              </span>
            </div>
            <div className="mt-3 flex items-center justify-between gap-4">
              <span className="text-gray-500 dark:text-gray-400">
                {t("playerProfile.projectedHealthyPlayers")}
              </span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {terminationPreview.squad_safety.healthy_players}/11
              </span>
            </div>
            {!terminationPreview.squad_safety.can_field_matchday_squad ? (
              <p className="mt-3 text-red-600 dark:text-red-300">
                {t("playerProfile.terminationUnsafe")}
              </p>
            ) : null}
          </div>
        ) : (
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("common.loading")}
          </p>
        )}

        {contractActionError ? (
          <p className="text-sm text-red-600 dark:text-red-300">
            {contractActionError}
          </p>
        ) : null}

        <div className="flex justify-end gap-2">
          <Button
            variant="ghost"
            onClick={onCancel}
            disabled={contractActionSubmitting}
          >
            {t("common.cancel")}
          </Button>
          <Button
            variant="outline"
            className="text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
            disabled={
              contractActionSubmitting ||
              !terminationPreview?.squad_safety.can_field_matchday_squad
            }
            onClick={onConfirm}
          >
            {t("playerProfile.confirmTerminateContract")}
          </Button>
        </div>
      </div>
    </DashboardModalFrame>
  );
}
