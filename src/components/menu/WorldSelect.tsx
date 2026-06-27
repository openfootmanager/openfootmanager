import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui";
import { X, ChevronRight, Globe, Users, ArrowLeft, Loader2, Trophy, Package } from "lucide-react";
import type { CareerStartPhase } from "./CreateManagerForm";

// ---------------------------------------------------------------------------
// Shared types — imported by MainMenu and PackageBuildStep
// ---------------------------------------------------------------------------

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

export interface PackageIssue {
  code: string;
  file: string;
  params: Record<string, string>;
}

export interface StackConflictInfo {
  severity: "warning" | "error";
  code: string;
  entityKind: string;
  entityId: string;
  packages: string[];
}

export interface PackageInfo {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  license: string;
  gameMinVersion: string;
  packageType: string;
  teamCount: number;
  playerCount: number;
  competitionCount: number;
  installedPath: string;
  logoDataUrl?: string;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const HISTORY_DEPTH_OPTIONS = [0, 6, 12, 24] as const;

function historyDepthOptionLabel(
  t: (key: string, options?: Record<string, unknown>) => string,
  value: (typeof HISTORY_DEPTH_OPTIONS)[number],
): string {
  if (value === 0) return t("worldSelect.historyDepth.none");
  return t("worldSelect.historyDepth.option", { count: value });
}

// ---------------------------------------------------------------------------
// Step indicator (shared with PackageBuildStep)
// ---------------------------------------------------------------------------

function StepIndicator({ current }: { current: 2 | 3 }) {
  const active = "flex items-center justify-center w-6 h-6 rounded-full bg-primary-500 text-white text-xs font-bold";
  const done = "flex items-center justify-center w-6 h-6 rounded-full bg-primary-500/30 text-primary-400 text-xs font-bold";
  const future = "flex items-center justify-center w-6 h-6 rounded-full bg-gray-200 dark:bg-navy-600 text-gray-400 dark:text-gray-500 text-xs font-bold";
  const filledLine = "h-0.5 flex-1 bg-primary-500";
  const emptyLine = "h-0.5 flex-1 bg-gray-200 dark:bg-navy-600";

  return (
    <div className="flex items-center gap-2 mb-1">
      <div className={done}>1</div>
      <div className={filledLine} />
      <div className={current === 2 ? active : done}>2</div>
      <div className={current === 3 ? filledLine : emptyLine} />
      <div className={current === 3 ? active : future}>3</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface GenerationStepProps {
  isStarting: boolean;
  startYear: number;
  startPhase: CareerStartPhase;
  historyDepthYears: number;
  onChangeHistoryDepthYears: (value: number) => void;
  onStart: () => void;
  onBack: () => void;
  onClose: () => void;
  /** Pre-filtered active packages from MainMenu state. */
  activePackages: PackageInfo[];
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function GenerationStep({
  isStarting,
  startYear,
  startPhase,
  historyDepthYears,
  onChangeHistoryDepthYears,
  onStart,
  onBack,
  onClose,
  activePackages,
}: GenerationStepProps) {
  const { t } = useTranslation();
  const historyDepthLabelId = useId();

  const hasActiveDatabases = activePackages.some((p) => p.packageType === "database");

  // Coverage totals across active database packages
  const dbPackages = activePackages.filter((p) => p.packageType === "database");
  const totalTeams = dbPackages.reduce((s, p) => s + p.teamCount, 0);
  const totalPlayers = dbPackages.reduce((s, p) => s + p.playerCount, 0);
  const totalCompetitions = dbPackages.reduce((s, p) => s + p.competitionCount, 0);

  return (
    <div className="flex flex-col gap-4">
      {/* Header */}
      <div className="flex justify-between items-center mb-2">
        <div className="flex items-center gap-2">
          <button
            onClick={onBack}
            className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {t("generation.title")}
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

      <StepIndicator current={3} />

      {/* Summary card */}
      <div className="rounded-xl border border-gray-200 bg-gray-50 p-3 text-sm text-gray-600 dark:border-navy-600 dark:bg-navy-700/60 dark:text-gray-200">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
            {t("worldSelect.summary.startYear")}
          </span>
          <span className="font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {startYear}
          </span>
          <span className="rounded-full bg-primary-500/10 px-2 py-0.5 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-primary-600 dark:text-primary-300">
            {hasActiveDatabases
              ? t("worldSelect.historyMode.reference")
              : t("worldSelect.historyMode.generated")}
          </span>
        </div>
        <p className="mt-2 text-xs uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
          {hasActiveDatabases
            ? t(`worldSelect.summary.${startPhase}.reference`, { year: startYear, count: historyDepthYears })
            : t(`worldSelect.summary.${startPhase}.generated`, { year: startYear, count: historyDepthYears })}
        </p>
      </div>

      {/* Database packages — coverage summary */}
      {hasActiveDatabases && (
        <div className="rounded-xl border border-gray-200 bg-white p-3 text-sm dark:border-navy-600 dark:bg-navy-700/60">
          <p className="font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-2">
            {t("generation.coverage")}
          </p>
          <div className="flex items-center gap-3 flex-wrap mb-2">
            <span className="text-[10px] font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400 flex items-center gap-1">
              <Globe className="w-3 h-3" />{t("worldSelect.teams", { count: totalTeams })}
            </span>
            <span className="text-[10px] font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400 flex items-center gap-1">
              <Users className="w-3 h-3" />{t("worldSelect.players", { count: totalPlayers })}
            </span>
            <span className="text-[10px] font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400 flex items-center gap-1">
              <Trophy className="w-3 h-3" />{t("worldSelect.competitions", { count: totalCompetitions })}
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {dbPackages.map((p) => (
              <span
                key={p.id}
                className="inline-flex items-center gap-1 text-[10px] font-heading uppercase tracking-wider rounded-full bg-primary-500/10 px-2 py-0.5 text-primary-600 dark:text-primary-300"
              >
                <Package className="w-2.5 h-2.5" />
                {p.name || p.id}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* History depth — only for random world */}
      {!hasActiveDatabases && (
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
                {t("worldSelect.historyDepth.hint")}
              </p>
            </div>
            <span className="rounded-full bg-accent-500/10 px-2 py-0.5 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-accent-600 dark:text-accent-300 flex-shrink-0">
              {t("worldSelect.historyDepth.applied", { count: historyDepthYears })}
            </span>
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
                  role="radio"
                  aria-checked={selected}
                  onClick={() => onChangeHistoryDepthYears(value)}
                  className={`rounded-xl border px-3 py-3 text-left transition-all ${
                    selected
                      ? "border-primary-500 bg-primary-50 text-primary-700 ring-1 ring-primary-400/30 dark:border-primary-500 dark:bg-primary-500/10 dark:text-primary-300"
                      : "border-gray-200 bg-gray-50 text-gray-700 hover:border-gray-300 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-200 dark:hover:border-navy-500"
                  }`}
                >
                  <span className="block font-heading font-bold uppercase tracking-wide">
                    {historyDepthOptionLabel(t, value)}
                  </span>
                  {value === 12 && (
                    <span className="block text-[10px] font-heading font-bold uppercase tracking-[0.2em] text-primary-600 dark:text-primary-300 mt-0.5">
                      {t("worldSelect.historyDepth.recommended")}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </div>
      )}

      <Button
        variant="primary"
        size="lg"
        className="w-full"
        iconRight={isStarting ? <Loader2 className="animate-spin" /> : <ChevronRight />}
        onClick={onStart}
        disabled={isStarting}
      >
        {isStarting ? t("worldSelect.creatingWorld") : t("worldSelect.startCareer")}
      </Button>
    </div>
  );
}
