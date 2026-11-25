import { useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { RecentProject } from "../../../store/packageEditorTypes";
import type { SamplePackage } from "../../menu/PackageEditor/sampleData";
import { Button, SegmentedControl } from "../../ui";
import { useWorldsLibrary } from "./useWorldsLibrary";
import WorldsSidebar from "./WorldsSidebar";
import WorldsTable from "./WorldsTable";
import {
  buildWorldRows,
  fallbackWorldRows,
  filterWorldRows,
  type WorldFilter,
  type WorldRow,
} from "./worldRows";

interface WorldsHomeProps {
  isBusy: boolean;
  errorMsg: string | null;
  recentProjects: readonly RecentProject[];
  samples: readonly SamplePackage[];
  onNewWorld: () => void;
  onOpenFile: () => void;
  onOpenFolder: () => void;
  onStartFromSample: (sample: SamplePackage) => void;
  /** Opens a project folder. */
  onOpenProject: (path: string) => void;
  /** Opens an installed package's `.ofm` archive, as an editable copy. */
  onOpenInstalled: (ofmPath: string) => void;
  onBack: () => void;
}

/**
 * The World Editor's home (the approved sidebar and library): the way in on the left, and "Your worlds"
 * on the right as one table of the projects the editor has opened and the packages installed, with an
 * All / Projects / Installed filter and a search. An empty library says how to start.
 */
export default function WorldsHome({
  isBusy,
  errorMsg,
  recentProjects,
  samples,
  onNewWorld,
  onOpenFile,
  onOpenFolder,
  onStartFromSample,
  onOpenProject,
  onOpenInstalled,
  onBack,
}: WorldsHomeProps) {
  const { t } = useTranslation();
  const filterLabelId = useId();
  const [filter, setFilter] = useState<WorldFilter>("all");
  const [query, setQuery] = useState("");
  const { library, loading, failed, retry } = useWorldsLibrary(
    recentProjects.map((project) => project.path),
  );

  // When the library could not be read the editor's own record of its projects still lists them, and
  // they still open: only the counts are unknown.
  const rows = useMemo(() => {
    if (failed) return fallbackWorldRows(recentProjects);
    return library ? buildWorldRows(library, recentProjects) : [];
  }, [library, failed, recentProjects]);
  const shown = filterWorldRows(rows, filter, query);
  // An unreadable library is unknown, not empty: it does not say how to start.
  let emptyText = t("common.noResults");
  if (rows.length === 0) {
    emptyText = failed
      ? t("worldEditor.home.libraryFailedEmpty")
      : t("worldEditor.home.emptyLibrary");
  }

  function open(row: WorldRow) {
    if (row.kind === "installed") onOpenInstalled(row.path);
    else onOpenProject(row.path);
  }

  return (
    <div className="flex min-h-screen flex-col bg-page text-fg md:flex-row">
      <WorldsSidebar
        busy={isBusy}
        samples={samples}
        onNewWorld={onNewWorld}
        onOpenFile={onOpenFile}
        onOpenFolder={onOpenFolder}
        onStartFromSample={onStartFromSample}
        onBack={onBack}
      />
      <main className="flex min-w-0 flex-1 flex-col gap-4 p-6 md:h-screen md:overflow-hidden">
        <div className="flex flex-wrap items-center gap-3.5">
          <h1 className="font-heading text-title font-bold uppercase tracking-label">
            {t("worldEditor.home.yourWorlds")}
          </h1>
          <span id={filterLabelId} className="sr-only">
            {t("worldEditor.home.show")}
          </span>
          <SegmentedControl
            labelledBy={filterLabelId}
            value={filter}
            onChange={setFilter}
            options={[
              { value: "all", label: t("worldEditor.home.filterAll") },
              { value: "projects", label: t("worldEditor.home.filterProjects") },
              { value: "installed", label: t("worldEditor.home.filterInstalled") },
            ]}
          />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("worldEditor.home.search")}
            aria-label={t("worldEditor.home.search")}
            className="ml-auto h-9 w-56 rounded-lg border border-line-strong bg-panel px-3 text-body text-fg placeholder:text-fg-subtle focus:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          />
        </div>

        {failed ? (
          <div
            role="alert"
            className="flex flex-wrap items-center gap-3 rounded-lg border border-line bg-warning-tint px-4 py-3 text-body text-warning"
          >
            <span className="min-w-0 flex-1">{t("worldEditor.home.libraryFailed")}</span>
            <Button size="sm" variant="outline" disabled={loading} onClick={retry}>
              {t("common.retry")}
            </Button>
          </div>
        ) : null}

        {errorMsg ? (
          <p
            role="alert"
            className="rounded-lg border border-line bg-danger-tint px-4 py-3 text-body text-danger"
          >
            {errorMsg}
          </p>
        ) : null}

        {/* Said aloud as the filter and the search change what the table lists. */}
        <p role="status" className="sr-only">
          {t("worldEditor.home.resultCount", { count: shown.length })}
        </p>
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-line bg-panel shadow-card">
          <div className="min-h-0 flex-1 overflow-auto">
            <WorldsTable
              rows={shown}
              busy={isBusy}
              loading={loading && !failed}
              empty={emptyText}
              onOpen={open}
            />
          </div>
          <p className="shrink-0 border-t border-line bg-sunken px-4 py-2.5 text-caption text-fg-subtle">
            {t("worldEditor.home.installedNote")}
          </p>
        </div>
      </main>
    </div>
  );
}
