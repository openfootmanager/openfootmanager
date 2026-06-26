import { useId, useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui";
import { X, ChevronRight, Globe, Shuffle, Users, ArrowLeft, Loader2, Package, PackagePlus, Trash2, Trophy, AlertTriangle, AlertCircle } from "lucide-react";
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
  game_min_version: string;
  package_type: string;
  team_count: number;
  player_count: number;
  competition_count: number;
  installed_path: string;
  logo_data_url?: string;
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
  onStart: () => void;
  onBack: () => void;
  onClose: () => void;
  installedPackages?: PackageInfo[];
  activePackageIds?: string[];
  onTogglePackage?: (id: string) => void;
  onInstallPackage?: () => void;
  onUninstallPackage?: (id: string) => void;
  isInstallingPackage?: boolean;
  packageStackErrors?: PackageIssue[];
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
  historyDepthYears, onSelectWorld, onChangeHistoryDepthYears, onStart, onBack, onClose,
  installedPackages, activePackageIds, onTogglePackage, onInstallPackage, onUninstallPackage,
  isInstallingPackage, packageStackErrors,
}: WorldSelectProps) {
  const { t } = useTranslation();
  const historyDepthLabelId = useId();
  const [stackConflicts, setStackConflicts] = useState<StackConflictInfo[]>([]);

  useEffect(() => {
    if (!activePackageIds || activePackageIds.length < 2) {
      setStackConflicts([]);
      return;
    }
    let cancelled = false;
    invoke<StackConflictInfo[]>("check_package_stack", { packageIds: activePackageIds })
      .then((conflicts) => { if (!cancelled) setStackConflicts(conflicts); })
      .catch((err) => {
        if (cancelled) return;
        setStackConflicts([{
          severity: "error",
          code: typeof err === "string" ? err : "be.error.package.invalid",
          entityKind: "",
          entityId: "",
          packages: [],
        }]);
      });
    return () => { cancelled = true; };
  }, [activePackageIds]);

  const hasPackageStackErrors = (packageStackErrors?.length ?? 0) > 0;
  const stackConflictErrors = stackConflicts.filter((c) => c.severity === "error");
  const stackConflictWarnings = stackConflicts.filter((c) => c.severity === "warning");
  const hasStackConflictErrors = stackConflictErrors.length > 0;
  const activePackages = installedPackages?.filter((p) => activePackageIds?.includes(p.id)) ?? [];
  const hasPatchOnlyPackages =
    activePackages.length > 0 &&
    activePackages.every((p) => p.package_type !== "database");
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
              <div className={`w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5 ${db.id === "random" ? "bg-accent-500/10 text-accent-500" : "bg-primary-500/10 text-primary-500"}`}>
                {db.id === "random" ? <Shuffle className="w-5 h-5" /> : <Globe className="w-5 h-5" />}
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

      {/* Installed .ofm packages section */}
      {(onInstallPackage || (installedPackages && installedPackages.length > 0)) && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <p className="font-heading font-bold uppercase tracking-[0.18em] text-sm text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
              <Package className="w-4 h-4" />
              {t("worldSelect.installedPackages")}
            </p>
            {onInstallPackage && (
              <button
                onClick={onInstallPackage}
                disabled={isInstallingPackage}
                className="flex items-center gap-1 text-xs font-heading font-bold uppercase tracking-wider text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 disabled:opacity-50 transition-colors"
              >
                {isInstallingPackage ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <PackagePlus className="w-3.5 h-3.5" />
                )}
                {t("worldSelect.installPackage")}
              </button>
            )}
          </div>

          {installedPackages && installedPackages.length === 0 && (
            <p className="text-xs text-gray-400 dark:text-gray-500 text-center py-2">
              {t("worldSelect.noInstalledPackages")}
            </p>
          )}

          {installedPackages && installedPackages.map(pkg => {
            const isActive = activePackageIds?.includes(pkg.id) ?? false;
            return (
              <div
                key={pkg.id}
                className={`flex items-start gap-3 w-full p-3 rounded-xl border transition-all duration-200 ${
                  isActive
                    ? "bg-accent-50 dark:bg-accent-500/10 border-accent-400 dark:border-accent-500 ring-1 ring-accent-400/30"
                    : "bg-white dark:bg-navy-700 border-gray-200 dark:border-navy-600"
                }`}
              >
                <button
                  type="button"
                  onClick={() => onTogglePackage?.(pkg.id)}
                  className={`w-5 h-5 rounded border-2 flex-shrink-0 mt-0.5 flex items-center justify-center transition-colors ${
                    isActive
                      ? "bg-accent-500 border-accent-500"
                      : "border-gray-300 dark:border-navy-500"
                  }`}
                  aria-checked={isActive}
                  role="checkbox"
                >
                  {isActive && <div className="w-2 h-2 rounded-sm bg-white" />}
                </button>
                {pkg.logo_data_url ? (
                  <img src={pkg.logo_data_url} alt="" className="w-8 h-8 rounded object-contain flex-shrink-0 mt-0.5 border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800" />
                ) : (
                  <div className="w-8 h-8 rounded bg-primary-500/10 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <Package className="w-4 h-4 text-primary-500" />
                  </div>
                )}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-1.5 flex-wrap">
                    <p className="font-heading font-bold text-sm uppercase tracking-wide text-gray-800 dark:text-gray-200 truncate">
                      {pkg.name || pkg.id}
                    </p>
                    {pkg.package_type && pkg.package_type !== "database" && (
                      <span className="text-[9px] font-heading uppercase tracking-wider px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-400 flex-shrink-0">
                        {pkg.package_type}
                      </span>
                    )}
                  </div>
                  <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">
                    {pkg.author && t("worldSelect.packageAuthor", { author: pkg.author })}
                    {pkg.author && pkg.version && " · "}
                    {pkg.version && t("worldSelect.packageVersion", { version: pkg.version })}
                    {pkg.license && ` · ${pkg.license}`}
                  </p>
                  <div className="flex items-center gap-3 mt-1">
                    <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
                      <Globe className="w-3 h-3" />{t("worldSelect.teams", { count: pkg.team_count })}
                    </span>
                    <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
                      <Users className="w-3 h-3" />{t("worldSelect.players", { count: pkg.player_count })}
                    </span>
                    <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
                      <Trophy className="w-3 h-3" />{t("worldSelect.competitions", { count: pkg.competition_count })}
                    </span>
                  </div>
                </div>
                {onUninstallPackage && (
                  <button
                    onClick={() => onUninstallPackage(pkg.id)}
                    className="text-gray-400 hover:text-red-500 transition-colors flex-shrink-0 mt-0.5"
                    title={t("worldSelect.removePackage")}
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                )}
              </div>
            );
          })}

          {stackConflictWarnings.length > 0 && (
            <div className="rounded-xl border border-amber-300 dark:border-amber-500/40 bg-amber-50 dark:bg-amber-500/10 p-3 text-xs">
              <p className="font-heading font-bold uppercase tracking-wider text-amber-700 dark:text-amber-400 mb-1 flex items-center gap-1">
                <AlertTriangle className="w-3.5 h-3.5" />
                {t("worldSelect.stackConflictWarnings", { count: stackConflictWarnings.length })}
              </p>
              <ul className="list-disc pl-4 space-y-0.5 text-amber-700 dark:text-amber-300">
                {stackConflictWarnings.map((c, i) => (
                  <li key={i}>{t(c.code, { entityKind: c.entityKind, entityId: c.entityId, packages: c.packages.join(", ") })}</li>
                ))}
              </ul>
            </div>
          )}
          {(hasPackageStackErrors || hasStackConflictErrors) && (
            <div className="rounded-xl border border-red-300 dark:border-red-500/40 bg-red-50 dark:bg-red-500/10 p-3 text-xs">
              <p className="font-heading font-bold uppercase tracking-wider text-red-600 dark:text-red-400 mb-1 flex items-center gap-1">
                <AlertCircle className="w-3.5 h-3.5" />
                {t("worldSelect.packageStackErrors")}
              </p>
              <ul className="list-disc pl-4 space-y-0.5 text-red-600 dark:text-red-300">
                {packageStackErrors?.map((issue, index) => (
                  <li key={`issue-${index}`}>
                    {issue.file ? `[${issue.file}] ` : ""}
                    {t(issue.code, issue.params)}
                  </li>
                ))}
                {stackConflictErrors.map((c, i) => (
                  <li key={`conflict-${i}`}>{t(c.code, { entityKind: c.entityKind, entityId: c.entityId, packages: c.packages.join(", ") })}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      <Button
        variant="primary"
        size="lg"
        className="w-full"
        iconRight={isStarting ? <Loader2 className="animate-spin" /> : <ChevronRight />}
        onClick={onStart}
        disabled={isStarting || hasPackageStackErrors || hasStackConflictErrors || hasPatchOnlyPackages}
      >
        {isStarting ? t('worldSelect.creatingWorld') : t('worldSelect.startCareer')}
      </Button>
    </div>
  );
}
