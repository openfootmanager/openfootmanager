import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import DashboardModalFrame from "./dashboard/DashboardModalFrame";
import { Button } from "./ui";

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

  if (!open) {
    return null;
  }

  return (
    <DashboardModalFrame maxWidthClassName="max-w-md">
      <div className="space-y-4" data-testid="switch-club-confirm-modal">
        <div>
          <h3 className="text-lg font-heading font-bold text-gray-900 dark:text-gray-100">
            {t("jobs.switchConfirmTitle", "Leave your current club?")}
          </h3>
          <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">
            {t(
              "jobs.switchConfirmBody",
              "Accepting this opportunity will end your tenure at {{currentClub}} and move you to {{newClub}}.",
              {
                currentClub: currentClubName,
                newClub: newClubName,
              },
            )}
          </p>
        </div>
        <div className="flex items-center justify-end gap-3">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onCancel}
            disabled={busy}
          >
            {t("common.cancel", "Cancel")}
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={onConfirm}
            disabled={busy}
            data-testid="switch-club-confirm"
          >
            {t("jobs.switchConfirmAccept", "Accept new role")}
          </Button>
        </div>
      </div>
    </DashboardModalFrame>
  );
}
