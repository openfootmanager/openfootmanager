import { useTranslation } from "react-i18next";
import { ArrowLeft, Package, FolderOpen, Loader2 } from "lucide-react";

interface HomeViewProps {
  isBusy: boolean;
  errorMsg: string | null;
  onBack: () => void;
  onNewPackage: () => void;
  onOpenPackage: () => void;
}

export function HomeView({ isBusy, errorMsg, onBack, onNewPackage, onOpenPackage }: HomeViewProps) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 mb-2">
        <button
          onClick={onBack}
          className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
          {t("packageEditor.title")}
        </h2>
      </div>

      {errorMsg && (
        <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/30 rounded-lg px-3 py-2">
          {errorMsg}
        </div>
      )}

      <button
        onClick={onNewPackage}
        disabled={isBusy}
        className="group flex items-center justify-between w-full p-4 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl transition-all duration-300 shadow-md hover:shadow-lg disabled:opacity-60 disabled:cursor-not-allowed"
      >
        <div className="flex items-center gap-3">
          {isBusy ? (
            <Loader2 className="w-6 h-6 animate-spin" />
          ) : (
            <Package className="w-6 h-6" />
          )}
          <span className="font-heading font-bold text-lg uppercase tracking-wide">
            {t("packageEditor.newPackage")}
          </span>
        </div>
      </button>

      <button
        onClick={onOpenPackage}
        disabled={isBusy}
        className="group flex items-center justify-between w-full p-4 bg-white dark:bg-navy-700 hover:bg-gray-50 dark:hover:bg-navy-600 text-gray-800 dark:text-gray-200 rounded-xl transition-all duration-300 border border-gray-200 dark:border-navy-600 hover:border-accent-400 dark:hover:border-accent-400 shadow-sm disabled:opacity-60 disabled:cursor-not-allowed"
      >
        <div className="flex items-center gap-3">
          <FolderOpen className="w-6 h-6 text-accent-500 dark:text-accent-400" />
          <span className="font-heading font-bold text-lg uppercase tracking-wide">
            {t("packageEditor.openPackage")}
          </span>
        </div>
      </button>
    </div>
  );
}
