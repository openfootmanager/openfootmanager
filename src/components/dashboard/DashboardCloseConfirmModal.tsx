import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardCloseConfirmModalProps {
  onCancel: () => void;
  onQuitWithoutSave: () => void;
  onSaveAndQuit: () => void;
}

export default function DashboardCloseConfirmModal({
  onCancel,
  onQuitWithoutSave,
  onSaveAndQuit,
}: DashboardCloseConfirmModalProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <DashboardModalFrame maxWidthClassName="max-w-sm">
      <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
        {t("closeConfirm.title")}
      </h3>
      <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
        {t("closeConfirm.message")}
      </p>
      <div className="mt-6 flex flex-col gap-2">
        <button
          onClick={onSaveAndQuit}
          className="w-full rounded-lg bg-primary-500 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-white transition-colors hover:bg-primary-600"
        >
          {t("closeConfirm.saveQuit")}
        </button>
        <button
          onClick={onQuitWithoutSave}
          className="w-full rounded-lg bg-red-500 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-white transition-colors hover:bg-red-600"
        >
          {t("closeConfirm.quitNoSave")}
        </button>
        <button
          onClick={onCancel}
          className="w-full rounded-lg bg-gray-100 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-gray-700 transition-colors hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-300 dark:hover:bg-navy-600"
        >
          {t("common.cancel")}
        </button>
      </div>
    </DashboardModalFrame>
  );
}
