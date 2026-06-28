import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import i18n, { SUPPORTED_LANGUAGES } from "../../i18n";
import {
  ArrowLeft, CheckCircle, Package, Loader2, AlertCircle,
  Undo2, Redo2, Save, ToggleLeft, ToggleRight,
} from "lucide-react";
import { Select } from "../ui/Select";
import { ThemeToggle } from "../ui/ThemeToggle";

export type SaveState = "idle" | "saving" | "saved" | "error";

interface WorldEditorTopBarProps {
  packageName: string;
  packageDir: string;
  saveState: SaveState;
  isBusy: boolean;
  issueCount: number;
  autoSave: boolean;
  canUndo: boolean;
  canRedo: boolean;
  isDirty: boolean;
  onValidate: () => void;
  onBuild: () => void;
  onSave: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onToggleAutoSave: () => void;
}

export function WorldEditorTopBar({
  packageName,
  packageDir,
  saveState,
  isBusy,
  issueCount,
  autoSave,
  canUndo,
  canRedo,
  isDirty,
  onValidate,
  onBuild,
  onSave,
  onUndo,
  onRedo,
  onToggleAutoSave,
}: WorldEditorTopBarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <div className="flex-shrink-0 h-18 flex items-center justify-between px-4 gap-2 border-b border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800">
      {/* Left: back + identity */}
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
          <span className="text-xs text-gray-400 dark:text-gray-500 truncate hidden md:block max-w-[200px]">
            {packageDir}
          </span>
        )}
        {isDirty && saveState === "idle" && (
          <span className="text-[10px] text-amber-500 dark:text-amber-400 font-medium flex-shrink-0">
            ●
          </span>
        )}
      </div>

      {/* Right: undo/redo + save indicator + actions */}
      <div className="flex items-center gap-2 flex-shrink-0">
        {/* Undo / Redo */}
        <div className="flex items-center gap-0.5">
          <button
            onClick={onUndo}
            disabled={!canUndo || isBusy}
            title={t("worldEditor.undo")}
            className="p-1.5 rounded-lg text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700 hover:text-gray-900 dark:hover:text-white transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
          >
            <Undo2 className="w-4 h-4" />
          </button>
          <button
            onClick={onRedo}
            disabled={!canRedo || isBusy}
            title={t("worldEditor.redo")}
            className="p-1.5 rounded-lg text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700 hover:text-gray-900 dark:hover:text-white transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
          >
            <Redo2 className="w-4 h-4" />
          </button>
        </div>

        <span className="w-px h-5 bg-gray-200 dark:bg-navy-600" />

        {/* Auto-save toggle */}
        <button
          onClick={onToggleAutoSave}
          title={autoSave ? t("worldEditor.autoSaveOn") : t("worldEditor.autoSaveOff")}
          className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
        >
          {autoSave ? (
            <ToggleRight className="w-4 h-4 text-primary-500" />
          ) : (
            <ToggleLeft className="w-4 h-4" />
          )}
          <span className="hidden sm:inline text-[11px]">{t("worldEditor.autoSave")}</span>
        </button>

        {/* Save indicator + manual save */}
        <button
          onClick={onSave}
          disabled={isBusy}
          title={t("worldEditor.save")}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-xs font-heading font-bold uppercase tracking-wider text-gray-700 dark:text-gray-200 hover:border-primary-400 dark:hover:border-primary-500 hover:text-primary-600 dark:hover:text-primary-400 transition-all disabled:opacity-40"
        >
          {saveState === "saving" ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : saveState === "saved" ? (
            <Save className="w-3.5 h-3.5 text-green-500" />
          ) : saveState === "error" ? (
            <AlertCircle className="w-3.5 h-3.5 text-red-500" />
          ) : (
            <Save className="w-3.5 h-3.5" />
          )}
          {saveState === "saving" ? (
            <span>{t("worldEditor.saving")}</span>
          ) : saveState === "saved" ? (
            <span className="text-green-600 dark:text-green-400">{t("worldEditor.saved")}</span>
          ) : saveState === "error" ? (
            <span className="text-red-500">{t("worldEditor.unsaved")}</span>
          ) : (
            <span>{t("worldEditor.save")}</span>
          )}
        </button>

        <span className="w-px h-5 bg-gray-200 dark:bg-navy-600" />

        <button
          onClick={onValidate}
          disabled={isBusy}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-xs font-heading font-bold uppercase tracking-wider text-gray-700 dark:text-gray-200 hover:border-primary-400 dark:hover:border-primary-500 hover:text-primary-600 dark:hover:text-primary-400 transition-all disabled:opacity-50"
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
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-gradient-to-r from-accent-500 to-accent-600 hover:from-accent-600 hover:to-accent-700 text-white text-xs font-heading font-bold uppercase tracking-wider transition-all disabled:opacity-50"
        >
          {isBusy ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Package className="w-3.5 h-3.5" />
          )}
          {t("worldEditor.build")}
        </button>

        <span className="w-px h-5 bg-gray-200 dark:bg-navy-600" />

        {/* Language picker */}
        <Select
          value={i18n.language}
          onChange={(e) => { void i18n.changeLanguage(e.target.value); }}
          selectSize="sm"
          title={t("settings.language")}
        >
          {SUPPORTED_LANGUAGES.map(({ code, labelKey }) => (
            <option key={code} value={code}>{t(labelKey)}</option>
          ))}
        </Select>

        {/* Theme toggle */}
        <ThemeToggle />
      </div>
    </div>
  );
}
