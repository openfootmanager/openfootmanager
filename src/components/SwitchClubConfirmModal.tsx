import { type JSX, useId } from "react";
import { useTranslation } from "react-i18next";

import { Button, Dialog } from "./ui";

interface SwitchClubConfirmModalProps {
  open: boolean;
  currentClubName: string;
  newClubName: string;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export default function SwitchClubConfirmModal({
  open,
  currentClubName,
  newClubName,
  busy = false,
  onCancel,
  onConfirm,
}: SwitchClubConfirmModalProps): JSX.Element | null {
  const { t } = useTranslation();
  const titleId = useId();

  if (!open) {
    return null;
  }

  return (
    <Dialog
      labelledBy={titleId}
      // While the switch runs, Cancel is disabled, and Escape asks for nothing either.
      onClose={busy ? undefined : onCancel}
      busy={busy}
      className="w-full max-w-md rounded-2xl p-6"
    >
      <div className="space-y-4" data-testid="switch-club-confirm-modal">
        <div>
          <h3
            id={titleId}
            className="text-lg font-heading font-bold text-gray-900 dark:text-gray-100"
          >
            {t("jobs.switchConfirmTitle")}
          </h3>
          <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">
            {t("jobs.switchConfirmBody", {
              currentClub: currentClubName,
              newClub: newClubName,
            })}
          </p>
        </div>
        <div className="flex items-center justify-end gap-3">
          <Button type="button" variant="outline" size="sm" onClick={onCancel} disabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={onConfirm}
            disabled={busy}
            data-testid="switch-club-confirm"
          >
            {t("jobs.switchConfirmAccept")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
