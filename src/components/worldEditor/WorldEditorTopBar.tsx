import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ArrowLeft, CheckCircle, Package, Loader2, Check, AlertCircle } from "lucide-react";

export type SaveState = "idle" | "saving" | "saved" | "error";

interface WorldEditorTopBarProps {
  packageName: string;
  packageDir: string;
  saveState: SaveState;
  isBusy: boolean;
  issueCount: number;
  onValidate: () => void;
  onBuild: () => void;
}

export function WorldEditorTopBar({
  packageName,
  packageDir,
  saveState,
  isBusy,
  issueCount,
  onValidate,
  onBuild,
}: WorldEditorTopBarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <div className="flex-shrink-0 h-12 flex items-center justify-between px-4 gap-4 border-b border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800">
      {/* Left: back + package identity */}
      <div className="flex items-center gap-3 min-w-0">
        <button
          onClick={() => navigate("/")}
          className="flex items-center gap-1.5 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors flex-shrink-0"
        >
          <ArrowLeft className="w-4 h-4" />
          {t("menu.mainMenu")}
        </button>
        <span className="text-gray-300 dark:text-navy-500 flex-shrink-0">·</span>
        <span className="font-heading font-bold text-sm uppercase tracking-wide text-gray-900 dark:text-white truncate">
          {packageName || t("worldEditor.title")}
        </span>
        {packageDir && (
          <span className="text-xs text-gray-400 dark:text-gray-500 truncate hidden md:block max-w-[240px]">
            {packageDir}
          </span>
        )}
      </div>

      {/* Right: save indicator + actions */}
      <div className="flex items-center gap-3 flex-shrink-0">
        {saveState === "saving" && (
          <span className="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500">
            <Loader2 className="w-3 h-3 animate-spin" />
            {t("worldEditor.saving")}
          </span>
        )}
        {saveState === "saved" && (
          <span className="flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
            <Check className="w-3 h-3" />
            {t("worldEditor.saved")}
          </span>
        )}
        {saveState === "error" && (
          <span className="flex items-center gap-1 text-xs text-red-500 dark:text-red-400">
            <AlertCircle className="w-3 h-3" />
            {t("worldEditor.unsaved")}
          </span>
        )}

        <button
          onClick={onValidate}
          disabled={isBusy}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-xs font-heading font-bold uppercase tracking-wider text-gray-700 dark:text-gray-200 hover:border-primary-400 dark:hover:border-primary-500 hover:text-primary-600 dark:hover:text-primary-400 transition-all disabled:opacity-50"
        >
          {isBusy ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : issueCount > 0 ? (
            <AlertCircle className="w-3.5 h-3.5 text-red-500" />
          ) : (
            <CheckCircle className="w-3.5 h-3.5" />
          )}
          {t("worldEditor.validate")}
          {issueCount > 0 && (
            <span className="ml-0.5 bg-red-500 text-white text-[10px] rounded-full px-1.5 py-0.5 leading-none">
              {issueCount}
            </span>
          )}
        </button>

        <button
          onClick={onBuild}
          disabled={isBusy}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-gradient-to-r from-accent-500 to-accent-600 hover:from-accent-600 hover:to-accent-700 text-white text-xs font-heading font-bold uppercase tracking-wider transition-all disabled:opacity-50"
        >
          {isBusy ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Package className="w-3.5 h-3.5" />
          )}
          {t("worldEditor.build")}
        </button>
      </div>
    </div>
  );
}
