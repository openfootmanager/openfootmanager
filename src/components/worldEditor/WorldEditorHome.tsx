import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  Package,
  FolderOpen,
  Loader2,
  Zap,
  Clock,
  ChevronRight,
  Package2,
} from "lucide-react";
import type { WorldMetaDef } from "../menu/PackageEditor/types";
import {
  SAMPLE_PACKAGES,
  type SamplePackage,
} from "../menu/PackageEditor/sampleData";
import type { PackageInfo } from "../menu/WorldSelect";

export interface RecentProject {
  path: string;
  name: string;
  openedAt: string;
}

type HomeView = "home" | "new-form";

function toSlug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
}

interface WorldEditorHomeProps {
  isBusy: boolean;
  errorMsg: string | null;
  recentProjects: RecentProject[];
  onNewPackage: (meta: WorldMetaDef, sample: SamplePackage | null) => void;
  onOpenPackage: () => void;
  onOpenRecent: (path: string) => void;
}

export function WorldEditorHome({
  isBusy,
  errorMsg,
  recentProjects,
  onNewPackage,
  onOpenPackage,
  onOpenRecent,
}: WorldEditorHomeProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const [view, setView] = useState<HomeView>("home");
  const [formName, setFormName] = useState("");
  const [formAuthor, setFormAuthor] = useState("");
  const [formDesc, setFormDesc] = useState("");
  const [pendingSample, setPendingSample] = useState<SamplePackage | null>(null);
  const [installedPackages, setInstalledPackages] = useState<PackageInfo[]>([]);

  useEffect(() => {
    invoke<PackageInfo[]>("list_installed_packages")
      .then(setInstalledPackages)
      .catch(() => {});
  }, []);

  const derivedSlug = toSlug(formName);

  function openNewForm(sample?: SamplePackage) {
    if (sample) {
      setFormName(sample.meta.name);
      setFormAuthor(sample.meta.author);
      setFormDesc(sample.meta.description);
      setPendingSample(sample);
    } else {
      setFormName("");
      setFormAuthor("");
      setFormDesc("");
      setPendingSample(null);
    }
    setView("new-form");
  }

  function handleCreate() {
    if (!derivedSlug) return;
    const meta: WorldMetaDef = {
      id: derivedSlug,
      name: formName.trim() || derivedSlug,
      description: formDesc.trim(),
      version: "1.0.0",
      author: formAuthor.trim(),
      license: "CC0-1.0",
      packageType: "database",
      gameMinVersion: "0.3.0",
      baseYear: new Date().getFullYear(),
      formatVersion: 1,
      defaultActiveRegions: [],
      defaultActiveCompetitions: pendingSample?.meta.defaultActiveCompetitions ?? [],
      logo: null,
    };
    onNewPackage(meta, pendingSample);
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Top bar (shared across views)
  // ──────────────────────────────────────────────────────────────────────────
  const topBar = (
    <div className="flex-shrink-0 h-12 flex items-center px-4 border-b border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800">
      <button
        onClick={() => (view === "new-form" ? setView("home") : navigate("/"))}
        className="flex items-center gap-1.5 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
      >
        <ArrowLeft className="w-4 h-4" />
        {view === "new-form" ? t("common.back") : t("menu.mainMenu")}
      </button>
    </div>
  );

  // ──────────────────────────────────────────────────────────────────────────
  // New-form view
  // ──────────────────────────────────────────────────────────────────────────
  if (view === "new-form") {
    return (
      <div className="min-h-screen flex flex-col bg-gray-50 dark:bg-navy-900">
        {topBar}
        <div className="flex-1 flex items-center justify-center p-8">
          <div className="w-full max-w-md">
            <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white mb-6">
              {t("worldEditor.newWorldForm")}
            </h2>

            {errorMsg && (
              <div className="mb-4 text-sm text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/30 rounded-xl px-4 py-3">
                {errorMsg}
              </div>
            )}

            <div className="flex flex-col gap-4">
              <div>
                <label className="block text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1">
                  {t("worldEditor.worldName")} *
                </label>
                <input
                  type="text"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                  placeholder={t("worldEditor.worldNamePlaceholder")}
                  autoFocus
                  className="w-full px-3 py-2 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-900 dark:text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1">
                  {t("worldEditor.worldId")}
                </label>
                <div className="flex items-center gap-2">
                  <div className="flex-1 px-3 py-2 rounded-lg border border-gray-100 dark:border-navy-700 bg-gray-50 dark:bg-navy-800 text-gray-500 dark:text-gray-400 text-sm font-mono truncate">
                    {derivedSlug || "—"}
                  </div>
                </div>
                <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                  {t("worldEditor.worldIdHint")}
                </p>
              </div>

              <div>
                <label className="block text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1">
                  {t("worldEditor.worldAuthor")}
                </label>
                <input
                  type="text"
                  value={formAuthor}
                  onChange={(e) => setFormAuthor(e.target.value)}
                  className="w-full px-3 py-2 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-900 dark:text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1">
                  {t("worldEditor.worldDesc")}
                </label>
                <textarea
                  value={formDesc}
                  onChange={(e) => setFormDesc(e.target.value)}
                  rows={3}
                  className="w-full px-3 py-2 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-900 dark:text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 resize-none"
                />
              </div>
            </div>

            <div className="flex justify-end gap-3 mt-6">
              <button
                onClick={() => setView("home")}
                disabled={isBusy}
                className="px-4 py-2 text-sm font-heading font-bold uppercase tracking-wide text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white transition-colors disabled:opacity-60"
              >
                {t("common.back")}
              </button>
              <button
                onClick={handleCreate}
                disabled={isBusy || !derivedSlug}
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl font-heading font-bold uppercase tracking-wide text-sm transition-all disabled:opacity-60 disabled:cursor-not-allowed"
              >
                {isBusy ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <ChevronRight className="w-4 h-4" />
                )}
                {isBusy ? t("worldEditor.creating") : t("worldEditor.createWorld")}
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Home view
  // ──────────────────────────────────────────────────────────────────────────
  return (
    <div className="min-h-screen flex flex-col bg-gray-50 dark:bg-navy-900">
      {topBar}

      <div className="flex-1 overflow-y-auto">
        <div className="max-w-lg mx-auto px-6 py-10 flex flex-col gap-8">
          {/* Hero */}
          <div className="text-center">
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

          {/* Primary actions */}
          <div className="flex flex-col gap-3">
            <button
              onClick={() => openNewForm()}
              disabled={isBusy}
              className="flex items-center gap-4 w-full p-5 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-2xl transition-all duration-200 shadow-md hover:shadow-lg disabled:opacity-60 disabled:cursor-not-allowed"
            >
              <Package className="w-7 h-7 flex-shrink-0" />
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

            <div className="flex flex-col gap-2">
              <p className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-400 dark:text-gray-500 flex items-center gap-1.5">
                <Zap className="w-3.5 h-3.5 text-amber-500" />
                {t("worldEditor.startFromExample")}
              </p>
              {SAMPLE_PACKAGES.map(
                (sample) => (
                  <button
                    key={sample.meta.id}
                    onClick={() => openNewForm(sample)}
                    disabled={isBusy}
                    className="flex items-start gap-3 w-full px-4 py-3 bg-white dark:bg-navy-800 hover:bg-amber-50 dark:hover:bg-navy-700 text-gray-800 dark:text-gray-200 rounded-xl transition-all duration-200 border border-gray-200 dark:border-navy-600 hover:border-amber-400 dark:hover:border-amber-500 shadow-sm disabled:opacity-60 disabled:cursor-not-allowed text-left"
                  >
                    <div className="flex-1 min-w-0">
                      <p className="font-heading font-bold uppercase tracking-wide text-sm truncate">
                        {sample.meta.name}
                      </p>
                      <p className="text-xs text-gray-400 dark:text-gray-500 mt-0.5 line-clamp-1">
                        {sample.meta.description}
                      </p>
                    </div>
                  </button>
                ),
              )}
            </div>
          </div>

          {/* Recent projects */}
          {recentProjects.length > 0 && (
            <div>
              <h3 className="flex items-center gap-2 text-xs font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wide mb-2">
                <Clock className="w-3.5 h-3.5" />
                {t("worldEditor.recentProjects")}
              </h3>
              <div className="flex flex-col gap-1">
                {recentProjects.map((proj) => (
                  <button
                    key={proj.path}
                    onClick={() => onOpenRecent(proj.path)}
                    disabled={isBusy}
                    className="flex items-center gap-3 w-full px-4 py-3 bg-white dark:bg-navy-800 hover:bg-gray-50 dark:hover:bg-navy-700 rounded-xl border border-gray-100 dark:border-navy-700 transition-colors text-left disabled:opacity-60"
                  >
                    <Package2 className="w-4 h-4 text-gray-400 dark:text-gray-500 flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-gray-800 dark:text-gray-200 truncate">
                        {proj.name}
                      </p>
                      <p className="text-xs text-gray-400 dark:text-gray-500 truncate font-mono">
                        {proj.path}
                      </p>
                    </div>
                    <ChevronRight className="w-4 h-4 text-gray-300 dark:text-gray-600 flex-shrink-0" />
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Installed packages */}
          {installedPackages.length > 0 && (
            <div>
              <h3 className="flex items-center gap-2 text-xs font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wide mb-2">
                <Package2 className="w-3.5 h-3.5" />
                {t("worldSelect.installedPackages")}
              </h3>
              <div className="flex flex-col gap-1">
                {installedPackages.map((pkg) => (
                  <div
                    key={pkg.id}
                    className="flex items-center gap-3 px-4 py-3 bg-white dark:bg-navy-800 rounded-xl border border-gray-100 dark:border-navy-700"
                  >
                    <Package2 className="w-4 h-4 text-gray-400 dark:text-gray-500 flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-gray-800 dark:text-gray-200 truncate">
                        {pkg.name}
                      </p>
                      <p className="text-xs text-gray-400 dark:text-gray-500">
                        v{pkg.version}
                        {pkg.author ? ` · ${pkg.author}` : ""}
                        {" · "}
                        {pkg.teamCount} teams
                      </p>
                    </div>
                    <button
                      onClick={() => onOpenRecent(pkg.installedPath)}
                      disabled={isBusy}
                      className="text-xs font-semibold text-primary-500 hover:text-primary-700 dark:hover:text-primary-300 transition-colors disabled:opacity-60 flex-shrink-0"
                    >
                      {t("worldEditor.openInEditor")}
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
