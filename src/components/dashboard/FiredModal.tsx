import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { ShieldX } from "lucide-react";
import { useGameStore } from "../../store/gameStore";

export default function FiredModal(): JSX.Element | null {
  const { t } = useTranslation();
  const { showFiredModal, setShowFiredModal, gameState } = useGameStore();

  if (!showFiredModal || !gameState) return null;

  const manager = gameState.manager;
  const lastEntry = manager.career_history?.length
    ? manager.career_history[manager.career_history.length - 1]
    : null;
  const teamName = lastEntry?.team_name || "";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="mx-4 w-full max-w-lg rounded-2xl bg-white shadow-2xl dark:bg-navy-800 dark:border dark:border-navy-700">
        {/* Header */}
        <div className="flex flex-col items-center pt-8 pb-4 px-6">
          <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-red-600 to-red-800 flex items-center justify-center mb-4 shadow-lg shadow-red-600/30">
            <ShieldX className="w-8 h-8 text-white" />
          </div>
          <h2 className="text-2xl font-heading font-bold text-gray-900 dark:text-gray-100 uppercase tracking-wide text-center">
            {t("sacked.title")}
          </h2>
          {teamName && <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">{teamName}</p>}
        </div>

        {/* Letter body */}
        <div className="px-8 pb-6">
          <div className="rounded-lg bg-gray-50 dark:bg-navy-900/50 p-5 border border-gray-200 dark:border-navy-700">
            <p className="text-sm text-gray-700 dark:text-gray-300 leading-relaxed whitespace-pre-line">
              {t("sacked.dismissalLetter", { team: teamName })}
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="px-8 pb-8">
          <button
            type="button"
            onClick={() => setShowFiredModal(false)}
            className="w-full rounded-xl bg-gray-700 dark:bg-navy-700 px-6 py-3 font-heading font-bold text-sm uppercase tracking-wider text-white transition-all hover:bg-gray-800 dark:hover:bg-navy-600 shadow-lg"
          >
            {t("dashboard.continue")}
          </button>
        </div>
      </div>
    </div>
  );
}
