import { useId } from "react";

import { Button, Dialog } from "../ui";
import { formatExactMoney } from "../../lib/helpers";
import type { ContractTerminationPreviewData } from "../../services/contractService";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileTerminationModalProps {
  show: boolean;
  playerName: string;
  t: TranslateFn;
  /** Null while the backend preview is still in flight. */
  preview: ContractTerminationPreviewData | null;
  errorMessage: string | null;
  submitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * Confirms releasing a player mid-contract, showing what it costs and whether
 * the squad could still field a matchday XI afterwards.
 */
export default function PlayerProfileTerminationModal({
  show,
  playerName,
  t,
  preview,
  errorMessage,
  submitting,
  onCancel,
  onConfirm,
}: PlayerProfileTerminationModalProps) {
  const titleId = useId();
  if (!show) {
    return null;
  }

  return (
    <Dialog
      labelledBy={titleId}
      // While the release is being submitted, Cancel is disabled, and Escape asks for nothing either.
      onClose={submitting ? undefined : onCancel}
      busy={submitting}
      className="w-full max-w-lg rounded-2xl p-6"
    >
      <div className="space-y-4">
        <div>
          <h2 id={titleId} className="font-heading text-heading font-bold text-fg">
            {t("playerProfile.terminateContractTitle")}
          </h2>
          <p className="mt-1 text-body text-fg-muted">
            {t("playerProfile.terminateContractBody", { name: playerName })}
          </p>
        </div>

        {preview ? (
          <div className="rounded-lg border border-line bg-sunken p-4 text-body">
            <div className="flex items-center justify-between gap-4">
              <span className="text-fg-muted">{t("playerProfile.terminationSeverance")}</span>
              <span className="font-semibold text-fg">
                {formatExactMoney(preview.severance_cost)}
              </span>
            </div>
            <div className="mt-3 flex items-center justify-between gap-4">
              <span className="text-fg-muted">{t("playerProfile.projectedHealthyPlayers")}</span>
              <span className="font-semibold text-fg">
                {preview.squad_safety.healthy_players}/11
              </span>
            </div>
            {!preview.squad_safety.can_field_matchday_squad ? (
              <p className="mt-3 text-danger">{t("playerProfile.terminationUnsafe")}</p>
            ) : null}
          </div>
        ) : (
          <p className="text-body text-fg-muted">{t("common.loading")}</p>
        )}

        {errorMessage ? <p className="text-body text-danger">{errorMessage}</p> : null}

        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onCancel} disabled={submitting}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="outline"
            className="hover:bg-danger-tint"
            disabled={submitting || !preview?.squad_safety.can_field_matchday_squad}
            onClick={onConfirm}
          >
            {/* The label carries the danger colour itself: the outline variant sets its own text colour,
                and which of two competing utilities wins is the order they are generated in. */}
            <span className="text-danger">{t("playerProfile.confirmTerminateContract")}</span>
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
