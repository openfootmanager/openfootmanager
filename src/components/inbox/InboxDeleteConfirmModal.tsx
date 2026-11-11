import { type JSX, useId } from "react";
import { useTranslation } from "react-i18next";

import { Button, Dialog } from "../ui";
import type { DeleteModalState } from "./inboxHelpers";

interface InboxDeleteConfirmModalProps {
  deleteModalState: DeleteModalState;
  isDeleting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export default function InboxDeleteConfirmModal({
  deleteModalState,
  isDeleting,
  onCancel,
  onConfirm,
}: InboxDeleteConfirmModalProps): JSX.Element | null {
  const { t } = useTranslation();
  const titleId = useId();

  if (!deleteModalState) {
    return null;
  }

  const title =
    deleteModalState.mode === "single"
      ? t("inbox.deleteMessageTitle")
      : t("inbox.deleteSelectedTitle");
  const message =
    deleteModalState.mode === "single"
      ? t("inbox.deleteMessageBody", { subject: deleteModalState.subject })
      : t("inbox.deleteSelectedBody", { count: deleteModalState.messageIds.length });

  return (
    <Dialog
      labelledBy={titleId}
      // While the delete runs, Cancel is disabled, and Escape asks for nothing either.
      onClose={isDeleting ? undefined : onCancel}
      busy={isDeleting}
      className="w-full max-w-md rounded-2xl p-6"
    >
      <div className="space-y-4" data-testid="inbox-delete-confirm-modal">
        <div>
          <h3
            id={titleId}
            className="text-lg font-heading font-bold text-gray-900 dark:text-gray-100"
          >
            {title}
          </h3>
          <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">{message}</p>
        </div>
        <div className="flex items-center justify-end gap-3">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onCancel}
            disabled={isDeleting}
          >
            {t("common.cancel")}
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={onConfirm}
            disabled={isDeleting}
            className="bg-red-500 hover:bg-red-600 active:bg-red-700 focus:ring-red-500"
            data-testid="inbox-confirm-delete"
          >
            {t("inbox.deleteAction")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
