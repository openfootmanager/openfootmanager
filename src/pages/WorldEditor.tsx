import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { MousePointerClick } from "lucide-react";
import { resolveBackendError } from "../utils/backendI18n";
import {
  emptyCompetition,
  emptyConfederation,
  emptyCountry,
  emptyMeta,
  emptyNamesDefinition,
  emptyPlayer,
  emptyTeam,
} from "../components/menu/PackageEditor/helpers";
import { useUndoRedo } from "../hooks/useUndoRedo";
import { useEntityEditor } from "../hooks/useEntityEditor";
import { MetadataForm } from "../components/menu/PackageEditor/MetadataForm";
import { TeamForm } from "../components/menu/PackageEditor/TeamForm";
import { ConfederationForm } from "../components/menu/PackageEditor/ConfederationForm";
import { CountryForm } from "../components/menu/PackageEditor/CountryForm";
import { PlayerForm } from "../components/menu/PackageEditor/PlayerForm";
import { NamesPoolForm } from "../components/menu/PackageEditor/NamesPoolForm";
import { CompetitionForm } from "../components/menu/PackageEditor/CompetitionForm";
import { TeamsTab } from "../components/menu/PackageEditor/TeamsTab";
import { PlayersTab } from "../components/menu/PackageEditor/PlayersTab";
import { ConfederationsTab } from "../components/menu/PackageEditor/ConfederationsTab";
import { CountriesTab } from "../components/menu/PackageEditor/CountriesTab";
import { NamesTab } from "../components/menu/PackageEditor/NamesTab";
import { CompetitionsTab } from "../components/menu/PackageEditor/CompetitionsTab";
import { IssueList } from "../components/menu/PackageEditor/IssueList";
import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  EditTab,
  NamePool,
  NamesDefinition,
  PackageProjectData,
  PlayerDef,
  TeamDef,
  WorldMetaDef,
} from "../components/menu/PackageEditor/types";
import { WorldEditorHome, type RecentProject } from "../components/worldEditor/WorldEditorHome";
import type { SamplePackage } from "../components/menu/PackageEditor/sampleData";
import { WorldEditorLayout } from "../components/worldEditor/WorldEditorLayout";
import { WorldEditorTopBar, type SaveState } from "../components/worldEditor/WorldEditorTopBar";
import { WorldEditorSidebar } from "../components/worldEditor/WorldEditorSidebar";
import { EntityListPanel } from "../components/worldEditor/EntityListPanel";

const AUTO_SAVE_KEY = "worldEditor.autoSave";
const RECENT_PROJECTS_KEY = "worldEditor.recentProjects";
const MAX_RECENT = 8;

function readRecentProjects(): RecentProject[] {
  try {
    const raw = localStorage.getItem(RECENT_PROJECTS_KEY);
    return raw ? (JSON.parse(raw) as RecentProject[]) : [];
  } catch {
    return [];
  }
}

type FormPanel =
  | "empty"
  | "metadata"
  | "team"
  | "confederation"
  | "country"
  | "player"
  | "names-pool"
  | "competition"
  | "issues";

interface EntitySnapshot {
  meta: WorldMetaDef;
  confederations: ConfederationDef[];
  countries: CountryDef[];
  teams: TeamDef[];
  players: PlayerDef[];
  names: NamesDefinition;
  competitions: CompetitionDef[];
}

function readAutoSave(): boolean {
  try {
    const stored = localStorage.getItem(AUTO_SAVE_KEY);
    return stored === null ? true : stored === "true";
  } catch {
    return true;
  }
}

export default function WorldEditor() {
  const { t } = useTranslation();

  const [projectDir, setProjectDir] = useState("");

  // Entity state
  const [meta, setMeta] = useState<WorldMetaDef>(emptyMeta());
  const [confederations, setConfederations] = useState<ConfederationDef[]>([]);
  const [countries, setCountries] = useState<CountryDef[]>([]);
  const [teams, setTeams] = useState<TeamDef[]>([]);
  const [players, setPlayers] = useState<PlayerDef[]>([]);
  const [names, setNames] = useState<NamesDefinition>(emptyNamesDefinition());
  const [competitions, setCompetitions] = useState<CompetitionDef[]>([]);
  const [issues, setIssues] = useState<PackageProjectData["issues"]>([]);

  // Layout state
  const [selectedSection, setSelectedSection] = useState<EditTab>("metadata");
  const [formPanel, setFormPanel] = useState<FormPanel>("metadata");
  const [sectionFormPanels, setSectionFormPanels] = useState<Partial<Record<EditTab, FormPanel>>>({});

  // Async state
  const [isBusy, setIsBusy] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [isDirty, setIsDirty] = useState(false);

  // Auto-save
  const [autoSave, setAutoSave] = useState<boolean>(readAutoSave);

  // Recent projects
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(readRecentProjects);

  // Names pool (bespoke: key-based, not index-based)
  const [editingPoolKey, setEditingPoolKey] = useState("");
  const [editingPool, setEditingPool] = useState<NamePool>({ first_names: [], last_names: [] });
  const [isNewPool, setIsNewPool] = useState(false);

  // ---------------------------------------------------------------------------
  // Snapshot helpers
  // ---------------------------------------------------------------------------

  function currentSnapshot(): EntitySnapshot {
    return { meta, confederations, countries, teams, players, names, competitions };
  }

  function applySnapshot(snapshot: EntitySnapshot) {
    setMeta(snapshot.meta);
    setConfederations(snapshot.confederations);
    setCountries(snapshot.countries);
    setTeams(snapshot.teams);
    setPlayers(snapshot.players);
    setNames(snapshot.names);
    setCompetitions(snapshot.competitions);
    setIsDirty(true);
    // Sync each editor's in-progress buffer from the restored snapshot so
    // the open form shows post-undo values rather than pre-undo ones.
    // Safe forward reference: applySnapshot is only called from keyboard
    // events, never during render, so editors are already initialised.
    teamEditor.syncEditing(snapshot.teams);
    confEditor.syncEditing(snapshot.confederations);
    countryEditor.syncEditing(snapshot.countries);
    playerEditor.syncEditing(snapshot.players);
    compEditor.syncEditing(snapshot.competitions);
  }

  // ---------------------------------------------------------------------------
  // Undo / redo
  // ---------------------------------------------------------------------------

  const { canUndo, canRedo, pushHistory, clearHistory, handleUndo, handleRedo } = useUndoRedo({
    getSnapshot: currentSnapshot,
    applySnapshot,
    enabled: !!projectDir,
    onDirty: () => setIsDirty(true),
  });

  const captureHistory = () => pushHistory(currentSnapshot());

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  function flashError(msg: string) {
    setErrorMsg(msg);
    setTimeout(() => setErrorMsg(null), 5000);
  }

  function loadProjectState(data: PackageProjectData) {
    setMeta(data.meta);
    setConfederations(data.confederations);
    setCountries(data.countries);
    setTeams(data.teams);
    setPlayers(data.players);
    setNames(data.names ?? emptyNamesDefinition());
    setCompetitions(data.competitions);
    setIssues(data.issues);
    clearHistory();
    setIsDirty(false);
  }

  function addRecentProject(path: string, name: string) {
    setRecentProjects((prev) => {
      const filtered = prev.filter((p) => p.path !== path);
      const updated = [{ path, name, openedAt: new Date().toISOString() }, ...filtered].slice(
        0,
        MAX_RECENT,
      );
      try {
        localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(updated));
      } catch { /* ignore */ }
      return updated;
    });
  }

  const persist = useCallback(async (overrides?: {
    meta?: WorldMetaDef;
    confederations?: ConfederationDef[];
    countries?: CountryDef[];
    teams?: TeamDef[];
    players?: PlayerDef[];
    names?: NamesDefinition;
    competitions?: CompetitionDef[];
  }) => {
    setSaveState("saving");
    try {
      await invoke("save_package_project", {
        dir: projectDir,
        meta: overrides?.meta ?? meta,
        confederations: overrides?.confederations ?? confederations,
        countries: overrides?.countries ?? countries,
        teams: overrides?.teams ?? teams,
        players: overrides?.players ?? players,
        names: overrides?.names ?? names,
        competitions: overrides?.competitions ?? competitions,
      });
      setSaveState("saved");
      setIsDirty(false);
      setTimeout(() => setSaveState("idle"), 2000);
    } catch (err) {
      setSaveState("error");
      flashError(resolveBackendError(err));
      throw err;
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectDir, meta, confederations, countries, teams, players, names, competitions]);

  function handleToggleAutoSave() {
    const next = !autoSave;
    setAutoSave(next);
    try { localStorage.setItem(AUTO_SAVE_KEY, String(next)); } catch { /* ignore */ }
  }

  async function handleManualSave() {
    setIsBusy(true);
    try {
      await persist();
    } catch {
      // persist handled error
    } finally {
      setIsBusy(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Section navigation
  // ---------------------------------------------------------------------------

  function handleSelectSection(section: EditTab) {
    // Save the current form panel for the section we're leaving
    setSectionFormPanels((prev) => ({ ...prev, [selectedSection]: formPanel }));
    setSelectedSection(section);
    if (section === "metadata") {
      setFormPanel("metadata");
    } else {
      // Restore previous form panel for this section, or default to empty
      setFormPanel(sectionFormPanels[section] ?? "empty");
    }
  }

  function handleShowIssues() {
    setFormPanel("issues");
  }

  // ---------------------------------------------------------------------------
  // Top-level project handlers
  // ---------------------------------------------------------------------------

  async function handleNewPackage(meta: WorldMetaDef, sample: SamplePackage | null) {
    setIsBusy(true);
    try {
      const dir = await invoke<string>("create_world_project", { slug: meta.id, meta });
      if (sample) {
        // Populate with sample entities
        await invoke("save_package_project", {
          dir,
          meta,
          confederations: sample.confederations,
          countries: sample.countries,
          teams: sample.teams,
          players: sample.players,
          names: sample.names,
          competitions: sample.competitions,
        });
      }
      const data = await invoke<PackageProjectData>("read_package_project", { dir });
      setProjectDir(dir);
      loadProjectState(data);
      addRecentProject(dir, meta.name || meta.id);
      setSelectedSection("metadata");
      setFormPanel("metadata");
    } catch (err) {
      flashError(resolveBackendError(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function openFromPath(path: string) {
    let dir: string;
    if (path.endsWith(".ofm")) {
      setIsBusy(true);
      try {
        dir = await invoke<string>("extract_ofm_for_editing", { ofmPath: path });
      } catch (err) {
        flashError(resolveBackendError(err));
        setIsBusy(false);
        return;
      }
    } else {
      dir = path;
      setIsBusy(true);
    }
    try {
      const data = await invoke<PackageProjectData>("read_package_project", { dir });
      setProjectDir(dir);
      loadProjectState(data);
      addRecentProject(dir, data.meta.name || data.meta.id);
      setSelectedSection("metadata");
      setFormPanel("metadata");
    } catch (err) {
      flashError(resolveBackendError(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function handleOpenPackage() {
    const selected = await open({
      directory: false,
      multiple: false,
      filters: [
        { name: "World Package", extensions: ["ofm"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (typeof selected === "string") {
      await openFromPath(selected);
      return;
    }
    // Fallback to directory picker if user cancelled or selected non-file
    const dirFallback = await open({ directory: true, multiple: false });
    if (typeof dirFallback === "string") {
      await openFromPath(dirFallback);
    }
  }

  async function handleValidate() {
    setIsBusy(true);
    try {
      await persist();
      const data = await invoke<PackageProjectData>("read_package_project", { dir: projectDir });
      setIssues(data.issues);
    } catch {
      // persist already handled the error
    } finally {
      setIsBusy(false);
    }
  }

  async function handleBuild() {
    const defaultName = `${meta.id || "package"}.ofm`;
    const outPath = await save({
      filters: [{ name: "OFM Package", extensions: ["ofm"] }],
      defaultPath: defaultName,
    });
    if (typeof outPath !== "string") return;
    setIsBusy(true);
    try {
      await persist();
      await invoke("build_ofm", { dir: projectDir, output: outPath });
    } catch {
      // persist already handled the error
    } finally {
      setIsBusy(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Entity editors (select / add / delete / save for each entity type)
  // ---------------------------------------------------------------------------

  const teamEditor = useEntityEditor({
    items: teams,
    setItems: setTeams,
    empty: emptyTeam,
    captureHistory,
    saveItems: (items) => persist({ teams: items }),
    autoSave,
    onOpen: () => setFormPanel("team"),
    onClose: () => setFormPanel("empty"),
    setIsBusy,
  });

  const confEditor = useEntityEditor({
    items: confederations,
    setItems: setConfederations,
    empty: emptyConfederation,
    captureHistory,
    saveItems: (items) => persist({ confederations: items }),
    autoSave,
    onOpen: () => setFormPanel("confederation"),
    onClose: () => setFormPanel("empty"),
    setIsBusy,
  });

  const countryEditor = useEntityEditor({
    items: countries,
    setItems: setCountries,
    empty: emptyCountry,
    captureHistory,
    saveItems: (items) => persist({ countries: items }),
    autoSave,
    onOpen: () => setFormPanel("country"),
    onClose: () => setFormPanel("empty"),
    setIsBusy,
  });

  const playerEditor = useEntityEditor({
    items: players,
    setItems: setPlayers,
    empty: emptyPlayer,
    captureHistory,
    saveItems: (items) => persist({ players: items }),
    autoSave,
    onOpen: () => setFormPanel("player"),
    onClose: () => setFormPanel("empty"),
    setIsBusy,
  });

  const compEditor = useEntityEditor({
    items: competitions,
    setItems: setCompetitions,
    empty: emptyCompetition,
    captureHistory,
    saveItems: (items) => persist({ competitions: items }),
    autoSave,
    onOpen: () => setFormPanel("competition"),
    onClose: () => setFormPanel("empty"),
    setIsBusy,
  });

  // ---------------------------------------------------------------------------
  // Country handlers (continued — country editor needs confederation list)
  // ---------------------------------------------------------------------------

  // Note: countryEditor.editing / handleSave etc. used in render below.
  // CountryForm also receives confederations for its dropdown.

  // ---------------------------------------------------------------------------
  // Names pool handlers (bespoke: key-based identity, rename-on-save)
  // ---------------------------------------------------------------------------

  function handleSelectPool(key: string) {
    setEditingPoolKey(key);
    setEditingPool({ ...names.pools[key] });
    setIsNewPool(false);
    setFormPanel("names-pool");
  }

  function handleAddPool() {
    setEditingPoolKey("");
    setEditingPool({ first_names: [], last_names: [] });
    setIsNewPool(true);
    setFormPanel("names-pool");
  }

  function handleDeletePool(key: string) {
    pushHistory(currentSnapshot());
    const updated: NamesDefinition = {
      ...names,
      pools: Object.fromEntries(Object.entries(names.pools).filter(([k]) => k !== key)),
    };
    setNames(updated);
    if (autoSave) void persist({ names: updated });
    if (editingPoolKey === key) setFormPanel("empty");
  }

  async function handleSavePool(key: string, pool: NamePool) {
    pushHistory(currentSnapshot());
    const updatedPools = isNewPool
      ? { ...names.pools, [key]: pool }
      : Object.fromEntries(
          Object.entries(names.pools).map(([k, v]) =>
            k === editingPoolKey ? [key, pool] : [k, v],
          ),
        );
    const updated: NamesDefinition = { ...names, pools: updatedPools };
    setNames(updated);
    setEditingPoolKey(key);
    setIsNewPool(false);
    if (autoSave) {
      setIsBusy(true);
      try {
        await persist({ names: updated });
      } catch {
        // non-fatal
      } finally {
        setIsBusy(false);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Home view (no project open)
  // ---------------------------------------------------------------------------

  if (!projectDir) {
    return (
      <WorldEditorHome
        isBusy={isBusy}
        errorMsg={errorMsg}
        recentProjects={recentProjects}
        onNewPackage={(meta, sample) => { void handleNewPackage(meta, sample); }}
        onOpenPackage={() => { void handleOpenPackage(); }}
        onOpenRecent={(path) => { void openFromPath(path); }}
      />
    );
  }

  // ---------------------------------------------------------------------------
  // 3-column editor layout
  // ---------------------------------------------------------------------------

  const showingIssues = formPanel === "issues";

  // Col 2: entity list (null when Metadata is selected)
  const listPanel =
    selectedSection === "metadata" ? null : (
      <EntityListPanel>
        {selectedSection === "teams" && (
          <TeamsTab
            teams={teams}
            onAdd={teamEditor.handleAdd}
            onEdit={teamEditor.handleSelect}
            onDelete={teamEditor.handleDelete}
            selectedIndex={formPanel === "team" ? teamEditor.editingIndex : null}
            onSelect={teamEditor.handleSelect}
          />
        )}
        {selectedSection === "players" && (
          <PlayersTab
            players={players}
            onAdd={playerEditor.handleAdd}
            onEdit={playerEditor.handleSelect}
            onDelete={playerEditor.handleDelete}
            selectedIndex={formPanel === "player" ? playerEditor.editingIndex : null}
            onSelect={playerEditor.handleSelect}
          />
        )}
        {selectedSection === "confederations" && (
          <ConfederationsTab
            confederations={confederations}
            onAdd={confEditor.handleAdd}
            onEdit={confEditor.handleSelect}
            onDelete={confEditor.handleDelete}
            selectedIndex={formPanel === "confederation" ? confEditor.editingIndex : null}
            onSelect={confEditor.handleSelect}
          />
        )}
        {selectedSection === "countries" && (
          <CountriesTab
            countries={countries}
            onAdd={countryEditor.handleAdd}
            onEdit={countryEditor.handleSelect}
            onDelete={countryEditor.handleDelete}
            selectedIndex={formPanel === "country" ? countryEditor.editingIndex : null}
            onSelect={countryEditor.handleSelect}
          />
        )}
        {selectedSection === "names" && (
          <NamesTab
            names={names}
            onAdd={handleAddPool}
            onEdit={handleSelectPool}
            onDelete={handleDeletePool}
            selectedKey={formPanel === "names-pool" ? editingPoolKey : null}
            onSelect={handleSelectPool}
          />
        )}
        {selectedSection === "competitions" && (
          <CompetitionsTab
            competitions={competitions}
            onAdd={compEditor.handleAdd}
            onEdit={compEditor.handleSelect}
            onDelete={compEditor.handleDelete}
            selectedIndex={formPanel === "competition" ? compEditor.editingIndex : null}
            onSelect={compEditor.handleSelect}
          />
        )}
      </EntityListPanel>
    );

  // Col 3: form or empty state
  const formContent = (() => {
    if (formPanel === "metadata") {
      return (
        <div className="max-w-4xl">
          <h2 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white mb-5">
            {t("worldEditor.metadata")}
          </h2>
          <MetadataForm
            meta={meta}
            onChange={(m) => { setMeta(m); setIsDirty(true); }}
            counts={{
              teams: teams.length,
              players: players.length,
              confederations: confederations.length,
              countries: countries.length,
              competitions: competitions.length,
              namePools: Object.keys(names.pools).length,
            }}
          />
          <button
            onClick={() => {
              pushHistory(currentSnapshot());
              void persist({ meta });
            }}
            disabled={isBusy}
            className="mt-6 px-5 py-2.5 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl font-heading font-bold uppercase tracking-wide text-sm transition-all disabled:opacity-60"
          >
            {t("common.save")}
          </button>
        </div>
      );
    }

    if (formPanel === "issues") {
      return (
        <div className="max-w-2xl">
          <h2 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white mb-5">
            {t("worldEditor.issuesTitle")}
          </h2>
          {issues.length === 0 ? (
            <p className="text-sm text-gray-400 dark:text-gray-500">
              {t("worldEditor.noIssues")}
            </p>
          ) : (
            <IssueList issues={issues} />
          )}
        </div>
      );
    }

    if (formPanel === "team") {
      return (
        <div className="max-w-lg">
          <TeamForm
            editingTeam={teamEditor.editing}
            editingTeamIndex={teamEditor.editingIndex}
            isBusy={isBusy}
            projectDir={projectDir || undefined}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void teamEditor.handleSave(); }}
            updateField={teamEditor.updateField}
          />
        </div>
      );
    }

    if (formPanel === "confederation") {
      return (
        <div className="max-w-lg">
          <ConfederationForm
            editing={confEditor.editing}
            editingIndex={confEditor.editingIndex}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void confEditor.handleSave(); }}
            updateField={confEditor.updateField}
          />
        </div>
      );
    }

    if (formPanel === "country") {
      return (
        <div className="max-w-lg">
          <CountryForm
            editing={countryEditor.editing}
            editingIndex={countryEditor.editingIndex}
            confederations={confederations}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void countryEditor.handleSave(); }}
            updateField={countryEditor.updateField}
          />
        </div>
      );
    }

    if (formPanel === "player") {
      return (
        <div className="max-w-lg">
          <PlayerForm
            editing={playerEditor.editing}
            editingIndex={playerEditor.editingIndex}
            isBusy={isBusy}
            teams={teams}
            projectDir={projectDir || undefined}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void playerEditor.handleSave(); }}
            updateField={playerEditor.updateField}
          />
        </div>
      );
    }

    if (formPanel === "names-pool") {
      return (
        <div className="max-w-lg">
          <NamesPoolForm
            poolKey={editingPoolKey}
            pool={editingPool}
            isNew={isNewPool}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={(key, pool) => { void handleSavePool(key, pool); }}
          />
        </div>
      );
    }

    if (formPanel === "competition") {
      return (
        <div className="max-w-2xl">
          <CompetitionForm
            editing={compEditor.editing}
            editingIndex={compEditor.editingIndex}
            isBusy={isBusy}
            teams={teams}
            projectDir={projectDir || undefined}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void compEditor.handleSave(); }}
            updateField={compEditor.updateField}
          />
        </div>
      );
    }

    // Empty state
    return (
      <div className="flex flex-col items-center justify-center h-full text-center gap-3">
        <MousePointerClick className="w-10 h-10 text-gray-300 dark:text-navy-600" />
        <p className="text-sm text-gray-400 dark:text-gray-500">
          {t("worldEditor.noItemSelected")}
        </p>
      </div>
    );
  })();

  return (
    <WorldEditorLayout
      topBar={
        <WorldEditorTopBar
          packageName={meta.name || meta.id}
          packageDir={projectDir}
          saveState={saveState}
          isBusy={isBusy}
          issueCount={issues.length}
          autoSave={autoSave}
          canUndo={canUndo}
          canRedo={canRedo}
          isDirty={isDirty}
          onValidate={() => { void handleValidate(); }}
          onBuild={() => { void handleBuild(); }}
          onSave={() => { void handleManualSave(); }}
          onUndo={handleUndo}
          onRedo={handleRedo}
          onToggleAutoSave={handleToggleAutoSave}
        />
      }
      sidebar={
        <WorldEditorSidebar
          selectedSection={selectedSection}
          onSelectSection={handleSelectSection}
          confederationCount={confederations.length}
          countryCount={countries.length}
          teamCount={teams.length}
          playerCount={players.length}
          namePoolCount={Object.keys(names.pools).length}
          competitionCount={competitions.length}
          issueCount={issues.length}
          onShowIssues={handleShowIssues}
          showingIssues={showingIssues}
        />
      }
      listPanel={listPanel}
      formPanel={formContent}
    />
  );
}
