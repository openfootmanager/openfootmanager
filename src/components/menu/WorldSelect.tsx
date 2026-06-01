import { useId, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui";
import { X, ChevronRight, Globe, Shuffle, Upload, Database, Users, ArrowLeft, Loader2 } from "lucide-react";
import { resolveBackendText } from "../../utils/backendI18n";
import type { CareerStartPhase } from "./CreateManagerForm";

export interface WorldDatabaseInfo {
  id: string;
  name: string;
  description: string;
  team_count: number;
  player_count: number;
  history_mode?: "generated" | "reference" | "hybrid";
  base_year?: number | null;
  snapshot_date?: string | null;
  source: string;
  path: string;
}

interface WorldSelectProps {
  worldDatabases: WorldDatabaseInfo[];
  selectedWorldId: string;
  isLoadingWorlds: boolean;
  isStarting: boolean;
  startYear: number;
  startPhase: CareerStartPhase;
  historyDepthYears: number;
  onSelectWorld: (id: string) => void;
  onChangeHistoryDepthYears: (value: number) => void;
  onImportFile: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onStart: () => void;
  onBack: () => void;
  onClose: () => void;
}

const HISTORY_DEPTH_OPTIONS = [0, 6, 12, 24] as const;

function historyDepthOptionLabel(
  t: (key: string, options?: Record<string, unknown>) => string,
  value: (typeof HISTORY_DEPTH_OPTIONS)[number],
): string {
  if (value === 0) {
    return t("worldSelect.historyDepth.none");
  }

  return t("worldSelect.historyDepth.option", { count: value });
}

function worldHistoryMode(db: WorldDatabaseInfo | undefined): "generated" | "reference" | "hybrid" {
  if (!db) return "generated";
  if (db.id === "random") return "generated";
  if (db.history_mode) return db.history_mode;
  return "hybrid";
}

export default function WorldSelect({
  worldDatabases, selectedWorldId, isLoadingWorlds, isStarting, startYear, startPhase,
  historyDepthYears, onSelectWorld, onChangeHistoryDepthYears, onImportFile, onStart, onBack, onClose,
}: WorldSelectProps) {
  const { t } = useTranslation();
  const historyDepthLabelId = useId();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const selectedWorld = worldDatabases.find((db) => db.id === selectedWorldId);
  const historyMode = worldHistoryMode(selectedWorld);
  const canConfigureGeneratedHistory = historyMode !== "reference";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex justify-between items-center mb-2">
        <div className="flex items-center gap-2">
          <button
            onClick={onBack}
            className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white transition-colors">
            {t('worldSelect.title')}
          </h2>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
        >
          <X className="w-5 h-5" />
        </button>
      </div>

      {/* Step indicator */}
      <div className="flex items-center gap-2 mb-1">
        <div className="flex items-center justify-center w-6 h-6 rounded-full bg-primary-500/30 text-primary-400 text-xs font-bold">1</div>
        <div className="h-0.5 flex-1 bg-primary-500" />
        <div className="flex items-center justify-center w-6 h-6 rounded-full bg-primary-500 text-white text-xs font-bold">2</div>
      </div>

      {/* World options */}
      <div className="flex flex-col gap-2 max-h-[45vh] overflow-y-auto pr-1">
        {isLoadingWorlds ? (
          <div className="text-gray-500 dark:text-gray-400 text-center py-4">{t('worldSelect.scanning')}</div>
        ) : (
          worldDatabases.map(db => (
            <button
              key={db.id}
              onClick={() => onSelectWorld(db.id)}
              className={`flex items-start gap-3 w-full p-3.5 rounded-xl border transition-all duration-200 text-left ${selectedWorldId === db.id
                ? "bg-primary-50 dark:bg-primary-500/10 border-primary-400 dark:border-primary-500 ring-1 ring-primary-400/30"
                : "bg-white dark:bg-navy-700 border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-500"
                }`}
            >
              <div className={`w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5 ${db.id === "random"
                ? "bg-accent-500/10 text-accent-500"
                : db.source === "imported"
                  ? "bg-purple-500/10 text-purple-500"
                  : "bg-primary-500/10 text-primary-500"
                }`}>
                {db.id === "random" ? <Shuffle className="w-5 h-5" /> :
                  db.source === "imported" ? <Upload className="w-5 h-5" /> :
                    <Database className="w-5 h-5" />}
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <p className={`font-heading font-bold text-sm uppercase tracking-wide ${selectedWorldId === db.id ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"
                    }`}>{db.id === "random" ? t('worldSelect.randomWorld') : resolveBackendText(db.name, db.name)}</p>
                  <span className="rounded-full bg-gray-100 px-2 py-0.5 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:bg-navy-600 dark:text-gray-300">
                    {t(`worldSelect.historyMode.${worldHistoryMode(db)}`)}
                  </span>
                </div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 line-clamp-2">{db.id === "random" ? t('worldSelect.randomDescription') : resolveBackendText(db.description, db.description)}</p>
                <div className="flex items-center gap-3 mt-1.5">
                  <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
                    <Globe className="w-3 h-3" />{t('worldSelect.teams', { count: db.team_count })}
                  </span>
                  <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
                    <Users className="w-3 h-3" />{t('worldSelect.players', { count: db.player_count })}
                  </span>
                </div>
              </div>
              {selectedWorldId === db.id && (
                <div className="w-5 h-5 rounded-full bg-primary-500 flex items-center justify-center flex-shrink-0 mt-1">
                  <div className="w-2 h-2 rounded-full bg-white" />
                </div>
              )}
            </button>
          ))
        )}
      </div>

      <div className="rounded-xl border border-gray-200 bg-gray-50 p-3 text-sm text-gray-600 dark:border-navy-600 dark:bg-navy-700/60 dark:text-gray-200">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
            {t("worldSelect.summary.startYear")}
          </span>
          <span className="font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {startYear}
          </span>
          <span className="rounded-full bg-primary-500/10 px-2 py-0.5 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-primary-600 dark:text-primary-300">
            {t(`worldSelect.historyMode.${historyMode}`)}
          </span>
        </div>
        <p className="mt-2 text-xs uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
          {t(`worldSelect.summary.${startPhase}.${historyMode}`, {
            year: startYear,
            count: historyDepthYears,
          })}
        </p>
      </div>

      <div className="rounded-xl border border-gray-200 bg-white p-3 text-sm dark:border-navy-600 dark:bg-navy-700/60">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p
              id={historyDepthLabelId}
              className="font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400"
            >
              {t("worldSelect.historyDepth.label")}
            </p>
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              {canConfigureGeneratedHistory
                ? t("worldSelect.historyDepth.hint")
                : t("worldSelect.historyDepth.reference")}
            </p>
          </div>
          {canConfigureGeneratedHistory ? (
            <span className="rounded-full bg-accent-500/10 px-2 py-0.5 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-accent-600 dark:text-accent-300">
              {t("worldSelect.historyDepth.applied", {
                count: historyDepthYears,
              })}
            </span>
          ) : null}
        </div>

        <div
          role="radiogroup"
          aria-labelledby={historyDepthLabelId}
          className="mt-3 grid grid-cols-2 gap-2"
        >
          {HISTORY_DEPTH_OPTIONS.map((value) => {
            const selected = historyDepthYears === value;

            return (
              <button
                key={value}
                type="button"
                disabled={!canConfigureGeneratedHistory}
                role="radio"
                aria-checked={selected}
                aria-disabled={!canConfigureGeneratedHistory}
                onClick={() => onChangeHistoryDepthYears(value)}
                className={`rounded-xl border px-3 py-3 text-left transition-all ${selected
                  ? "border-primary-500 bg-primary-50 text-primary-700 ring-1 ring-primary-400/30 dark:border-primary-500 dark:bg-primary-500/10 dark:text-primary-300"
                  : "border-gray-200 bg-gray-50 text-gray-700 hover:border-gray-300 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-200 dark:hover:border-navy-500"
                  } ${!canConfigureGeneratedHistory
                    ? "cursor-not-allowed opacity-60"
                    : ""
                  }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-heading font-bold uppercase tracking-wide">
                    {historyDepthOptionLabel(t, value)}
                  </span>
                  {value === 12 ? (
                    <span className="rounded-full bg-primary-500/10 px-2 py-0.5 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-primary-600 dark:text-primary-300">
                      {t("worldSelect.historyDepth.recommended")}
                    </span>
                  ) : null}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Import button */}
      <button
        onClick={() => fileInputRef.current?.click()}
        className="flex items-center justify-center gap-2 w-full py-2.5 border border-dashed border-gray-300 dark:border-navy-500 rounded-xl text-sm text-gray-500 dark:text-gray-400 hover:text-primary-500 dark:hover:text-primary-400 hover:border-primary-400 dark:hover:border-primary-500 transition-colors"
      >
        <Upload className="w-4 h-4" />
        <span className="font-heading font-bold uppercase tracking-wider">{t('worldSelect.importFile')}</span>
      </button>
      <input
        ref={fileInputRef}
        type="file"
        accept=".json"
        className="hidden"
        onChange={onImportFile}
      />

      <Button
        variant="primary"
        size="lg"
        className="w-full"
        iconRight={isStarting ? <Loader2 className="animate-spin" /> : <ChevronRight />}
        onClick={onStart}
        disabled={isStarting}
      >
        {isStarting ? t('worldSelect.creatingWorld') : t('worldSelect.startCareer')}
      </Button>
    </div>
  );
}
