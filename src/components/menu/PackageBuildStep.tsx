import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  X,
  ArrowLeft,
  ChevronRight,
  Package,
  PackagePlus,
  Trash2,
  Globe,
  Users,
  Trophy,
  Shuffle,
  AlertTriangle,
  AlertCircle,
  Loader2,
} from "lucide-react";
import { Button } from "../ui";
import type { PackageInfo, PackageIssue, StackConflictInfo } from "./WorldSelect";

interface PackageBuildStepProps {
  installedPackages: PackageInfo[];
  activePackageIds: string[];
  isInstallingPackage: boolean;
  packageStackErrors?: PackageIssue[];
  onTogglePackage: (id: string) => void;
  onInstallPackage: () => void;
  onUninstallPackage: (id: string) => void;
  onNext: () => void;
  onBack: () => void;
  onClose: () => void;
}

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

interface PackageCardProps {
  pkg: PackageInfo;
  isActive: boolean;
  onToggle: () => void;
  onUninstall: () => void;
}

function PackageCard({ pkg, isActive, onToggle, onUninstall }: PackageCardProps) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      onClick={onToggle}
      className={`flex items-start gap-3 w-full p-3 rounded-xl border transition-all duration-200 text-left ${
        isActive
          ? "bg-accent-50 dark:bg-accent-500/10 border-accent-400 dark:border-accent-500 ring-1 ring-accent-400/30"
          : "bg-white dark:bg-navy-700 border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-500"
      }`}
    >
      {/* Toggle indicator */}
      <span
        className={`w-5 h-5 rounded border-2 flex-shrink-0 mt-0.5 flex items-center justify-center transition-colors pointer-events-none ${
          isActive ? "bg-accent-500 border-accent-500" : "border-gray-300 dark:border-navy-500"
        }`}
        aria-hidden
      >
        {isActive && <span className="w-2 h-2 rounded-sm bg-white" />}
      </span>

      {/* Logo or icon */}
      {pkg.logoDataUrl ? (
        <img
          src={pkg.logoDataUrl}
          alt=""
          className="w-8 h-8 rounded object-contain flex-shrink-0 mt-0.5 border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800"
        />
      ) : (
        <div className="w-8 h-8 rounded bg-primary-500/10 flex items-center justify-center flex-shrink-0 mt-0.5">
          <Package className="w-4 h-4 text-primary-500" />
        </div>
      )}

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1.5 flex-wrap">
          <p className="font-heading font-bold text-sm uppercase tracking-wide text-gray-800 dark:text-gray-200 truncate">
            {pkg.name || pkg.id}
          </p>
        </div>
        <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">
          {pkg.author && t("worldSelect.packageAuthor", { author: pkg.author })}
          {pkg.author && pkg.version && " · "}
          {pkg.version && t("worldSelect.packageVersion", { version: pkg.version })}
          {pkg.license && ` · ${pkg.license}`}
        </p>
        <div className="flex items-center gap-3 mt-1">
          <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
            <Globe className="w-3 h-3" />{t("worldSelect.teams", { count: pkg.teamCount })}
          </span>
          <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
            <Users className="w-3 h-3" />{t("worldSelect.players", { count: pkg.playerCount })}
          </span>
          <span className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500 flex items-center gap-1">
            <Trophy className="w-3 h-3" />{t("worldSelect.competitions", { count: pkg.competitionCount })}
          </span>
        </div>
      </div>

      {/* Uninstall */}
      <span
        role="button"
        tabIndex={0}
        onClick={(e) => { e.stopPropagation(); onUninstall(); }}
        onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.stopPropagation(); onUninstall(); } }}
        className="text-gray-400 hover:text-red-500 transition-colors flex-shrink-0 mt-0.5 p-0.5 rounded"
        title={t("worldSelect.removePackage")}
        aria-label={t("worldSelect.removePackage")}
      >
        <Trash2 className="w-4 h-4" />
      </span>
    </button>
  );
}

export default function PackageBuildStep({
  installedPackages,
  activePackageIds,
  isInstallingPackage,
  packageStackErrors,
  onTogglePackage,
  onInstallPackage,
  onUninstallPackage,
  onNext,
  onBack,
  onClose,
}: PackageBuildStepProps) {
  const { t } = useTranslation();
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

  const dbPackages = installedPackages.filter((p) => p.packageType === "database");
  const patchPackages = installedPackages.filter((p) => p.packageType !== "database");
  const activePackages = installedPackages.filter((p) => activePackageIds.includes(p.id));
  const hasActiveDatabase = activePackages.some((p) => p.packageType === "database");
  const hasPatchOnly = activePackages.length > 0 && activePackages.every((p) => p.packageType !== "database");

  const hasPackageErrors = (packageStackErrors?.length ?? 0) > 0;
  const stackConflictErrors = stackConflicts.filter((c) => c.severity === "error");
  const stackConflictWarnings = stackConflicts.filter((c) => c.severity === "warning");
  const hasStackErrors = stackConflictErrors.length > 0;
  const canProceed = !hasPatchOnly && !hasPackageErrors && !hasStackErrors;

  // Show patches section when patches exist and at least one database is active,
  // or when patches exist and user may want to combine with random world.
  const showPatches = patchPackages.length > 0;

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
            {t("packageBuild.title")}
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

      <StepIndicator current={2} />

      {/* Scrollable content */}
      <div className="flex flex-col gap-4 max-h-[52vh] overflow-y-auto pr-1">

        {/* Databases section */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <p className="font-heading font-bold uppercase tracking-[0.18em] text-sm text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
              <Package className="w-4 h-4" />
              {t("packageBuild.databases")}
            </p>
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
          </div>

          {dbPackages.length === 0 ? (
            <p className="text-xs text-gray-400 dark:text-gray-500 text-center py-2">
              {t("packageBuild.noPackages")}
            </p>
          ) : (
            dbPackages.map((pkg) => (
              <PackageCard
                key={pkg.id}
                pkg={pkg}
                isActive={activePackageIds.includes(pkg.id)}
                onToggle={() => onTogglePackage(pkg.id)}
                onUninstall={() => onUninstallPackage(pkg.id)}
              />
            ))
          )}
        </div>

        {/* Patches section */}
        {showPatches && (
          <div className="space-y-2">
            <p className="font-heading font-bold uppercase tracking-[0.18em] text-sm text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
              <Package className="w-4 h-4" />
              {t("packageBuild.patches")}
            </p>
            {patchPackages.map((pkg) => (
              <PackageCard
                key={pkg.id}
                pkg={pkg}
                isActive={activePackageIds.includes(pkg.id)}
                onToggle={() => onTogglePackage(pkg.id)}
                onUninstall={() => onUninstallPackage(pkg.id)}
              />
            ))}
          </div>
        )}

        {/* Random world fallback notice */}
        {!hasActiveDatabase && (
          <div className="flex items-start gap-2.5 rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-700/60 p-3">
            <Shuffle className="w-4 h-4 text-accent-500 flex-shrink-0 mt-0.5" />
            <p className="text-xs text-gray-500 dark:text-gray-400">
              {t("packageBuild.randomFallback")}
            </p>
          </div>
        )}

        {/* Patch-only error */}
        {hasPatchOnly && (
          <div className="rounded-xl border border-amber-300 dark:border-amber-500/40 bg-amber-50 dark:bg-amber-500/10 p-3 text-xs">
            <p className="font-heading font-bold uppercase tracking-wider text-amber-700 dark:text-amber-400 flex items-center gap-1">
              <AlertCircle className="w-3.5 h-3.5" />
              {t("packageBuild.patchOnlyError")}
            </p>
          </div>
        )}

        {/* Stack conflict warnings */}
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

        {/* Stack errors */}
        {(hasPackageErrors || hasStackErrors) && (
          <div className="rounded-xl border border-red-300 dark:border-red-500/40 bg-red-50 dark:bg-red-500/10 p-3 text-xs">
            <p className="font-heading font-bold uppercase tracking-wider text-red-600 dark:text-red-400 mb-1 flex items-center gap-1">
              <AlertCircle className="w-3.5 h-3.5" />
              {t("worldSelect.packageStackErrors")}
            </p>
            <ul className="list-disc pl-4 space-y-0.5 text-red-600 dark:text-red-300">
              {packageStackErrors?.map((issue, i) => (
                <li key={`issue-${i}`}>
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

      {/* Next */}
      <Button
        variant="primary"
        size="lg"
        className="w-full"
        iconRight={<ChevronRight />}
        onClick={onNext}
        disabled={!canProceed}
      >
        {t("packageBuild.next")}
      </Button>
    </div>
  );
}
