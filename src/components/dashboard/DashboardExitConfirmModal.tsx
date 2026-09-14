import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardExitConfirmModalProps {
  onCancel: () => void;
  onConfirm: () => void;
}

export default function DashboardExitConfirmModal({
  onCancel,
  onConfirm,
}: DashboardExitConfirmModalProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <DashboardModalFrame maxWidthClassName="max-w-sm">
      <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
        {t("exitConfirm.title")}
      </h3>
      <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">{t("exitConfirm.message")}</p>
      <div className="mt-6 flex gap-3">
        <button
          type="button"
          onClick={onCancel}
          className="flex-1 rounded-lg bg-gray-100 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-gray-700 transition-colors hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-300 dark:hover:bg-navy-600"
        >
          {t("exitConfirm.cancel")}
        </button>
        <button
          type="button"
          onClick={onConfirm}
          className="flex-1 rounded-lg bg-red-500 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-white transition-colors hover:bg-red-600"
        >
          {t("exitConfirm.saveExit")}
        </button>
      </div>
    </DashboardModalFrame>
  );
}
