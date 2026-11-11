import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ArrowLeft, ChevronRight, Loader2 } from "lucide-react";
import type { WorldMetaDef } from "../menu/PackageEditor/types";
import { SAMPLE_PACKAGES, type SamplePackage } from "../menu/PackageEditor/sampleData";
import WorldsHome from "./home/WorldsHome";

import type { RecentProject } from "../../store/packageEditorTypes";
export type { RecentProject } from "../../store/packageEditorTypes";

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
  onOpenPackageFile: () => void;
  onOpenPackageFolder: () => void;
  onOpenRecent: (path: string) => void;
  /**
   * Open an installed package. Deliberately separate from `onOpenRecent`: a
   * recent project is a *directory*, an installed package is a `.ofm`
   * *archive*, and they are opened by different means. Routing both through one
   * callback is what made "Open in Editor" hand an archive path to the
   * directory reader, which found no files and opened an empty editor.
   */
  onOpenInstalled: (ofmPath: string) => void;
}

export function WorldEditorHome({
  isBusy,
  errorMsg,
  recentProjects,
  onNewPackage,
  onOpenPackageFile,
  onOpenPackageFolder,
  onOpenRecent,
  onOpenInstalled,
}: WorldEditorHomeProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const [view, setView] = useState<HomeView>("home");
  const [formName, setFormName] = useState("");
  const [formAuthor, setFormAuthor] = useState("");
  const [formDesc, setFormDesc] = useState("");
  const [pendingSample, setPendingSample] = useState<SamplePackage | null>(null);
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
        type="button"
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
                  // biome-ignore lint/a11y/noAutofocus: this is the new-world form, reached by clicking through from the home view.
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
                type="button"
                onClick={() => setView("home")}
                disabled={isBusy}
                className="px-4 py-2 text-sm font-heading font-bold uppercase tracking-wide text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white transition-colors disabled:opacity-60"
              >
                {t("common.back")}
              </button>
              <button
                type="button"
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

  // The home: the sidebar and the library of worlds.
  return (
    <WorldsHome
      isBusy={isBusy}
      errorMsg={errorMsg}
      recentProjects={recentProjects}
      samples={SAMPLE_PACKAGES}
      onNewWorld={() => openNewForm()}
      onOpenFile={onOpenPackageFile}
      onOpenFolder={onOpenPackageFolder}
      onStartFromSample={openNewForm}
      onOpenProject={onOpenRecent}
      onOpenInstalled={onOpenInstalled}
      onBack={() => navigate("/")}
    />
  );
}
