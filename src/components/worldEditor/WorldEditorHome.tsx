import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Package, FolderOpen, Loader2 } from "lucide-react";

interface WorldEditorHomeProps {
  isBusy: boolean;
  errorMsg: string | null;
  onNewPackage: () => void;
  onOpenPackage: () => void;
}

export function WorldEditorHome({
  isBusy,
  errorMsg,
  onNewPackage,
  onOpenPackage,
}: WorldEditorHomeProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <div className="min-h-screen flex flex-col bg-gray-50 dark:bg-navy-900">
      {/* Top bar */}
      <div className="flex-shrink-0 h-12 flex items-center px-4 border-b border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800">
        <button
          onClick={() => navigate("/")}
          className="flex items-center gap-1.5 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          {t("menu.mainMenu")}
        </button>
      </div>

      {/* Center content */}
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="w-full max-w-sm flex flex-col gap-4">
          <div className="text-center mb-4">
            <Package className="w-12 h-12 text-primary-500 mx-auto mb-3" />
            <h1 className="text-2xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
              {t("worldEditor.title")}
            </h1>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
              {t("worldEditor.homeSubtitle")}
            </p>
          </div>

          {errorMsg && (
            <div className="text-sm text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/30 rounded-xl px-4 py-3">
              {errorMsg}
            </div>
          )}

          <button
            onClick={onNewPackage}
            disabled={isBusy}
            className="flex items-center gap-4 w-full p-5 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-2xl transition-all duration-200 shadow-md hover:shadow-lg disabled:opacity-60 disabled:cursor-not-allowed"
          >
            {isBusy ? (
              <Loader2 className="w-7 h-7 animate-spin flex-shrink-0" />
            ) : (
              <Package className="w-7 h-7 flex-shrink-0" />
            )}
            <div className="text-left">
              <p className="font-heading font-bold text-lg uppercase tracking-wide">
                {t("worldEditor.newPackage")}
              </p>
              <p className="text-sm text-primary-100 mt-0.5">
                {t("worldEditor.newPackageDesc")}
              </p>
            </div>
          </button>

          <button
            onClick={onOpenPackage}
            disabled={isBusy}
            className="flex items-center gap-4 w-full p-5 bg-white dark:bg-navy-800 hover:bg-gray-50 dark:hover:bg-navy-700 text-gray-800 dark:text-gray-200 rounded-2xl transition-all duration-200 border border-gray-200 dark:border-navy-600 hover:border-accent-400 dark:hover:border-accent-400 shadow-sm disabled:opacity-60 disabled:cursor-not-allowed"
          >
            <FolderOpen className="w-7 h-7 text-accent-500 dark:text-accent-400 flex-shrink-0" />
            <div className="text-left">
              <p className="font-heading font-bold text-lg uppercase tracking-wide">
                {t("worldEditor.openPackage")}
              </p>
              <p className="text-sm text-gray-400 dark:text-gray-500 mt-0.5">
                {t("worldEditor.openPackageDesc")}
              </p>
            </div>
          </button>
        </div>
      </div>
    </div>
  );
}
