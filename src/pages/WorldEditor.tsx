import { useState, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { AlertCircle, CheckCircle2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { resolveBackendError } from "../utils/backendI18n";
import {
  emptyCompetition,
  emptyConfederation,
  emptyCountry,
  emptyMeta,
  emptyNamesDefinition,
  emptyPlayer,
  emptyStaff,
  emptyTeam,
} from "../components/menu/PackageEditor/helpers";
import { useUndoRedo } from "../hooks/useUndoRedo";
import { useEntityEditor } from "../hooks/useEntityEditor";
import { useNamesPoolEditor } from "../hooks/useNamesPoolEditor";
import { createWriteQueue } from "../lib/writeQueue";
import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  EditTab,
  NamesDefinition,
  PackageProjectData,
  PlayerDef,
  StaffDef,
  TeamDef,
  WorldMetaDef,
} from "../components/menu/PackageEditor/types";
import { WorldEditorHome, type RecentProject } from "../components/worldEditor/WorldEditorHome";
import type { SamplePackage } from "../components/menu/PackageEditor/sampleData";
import { WorldEditorLayout } from "../components/worldEditor/WorldEditorLayout";
import { WorldEditorTopBar, type SaveState } from "../components/worldEditor/WorldEditorTopBar";
import { WorldEditorSidebar } from "../components/worldEditor/WorldEditorSidebar";
import { WorldEditorFormPanel, type FormPanel } from "../components/worldEditor/WorldEditorFormPanel";
import { WorldEditorListContent } from "../components/worldEditor/WorldEditorListContent";

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

interface EntitySnapshot {
  meta: WorldMetaDef;
  confederations: ConfederationDef[];
  countries: CountryDef[];
  teams: TeamDef[];
  players: PlayerDef[];
  staff: StaffDef[];
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
  const [staff, setStaff] = useState<StaffDef[]>([]);
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
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [isDirty, setIsDirty] = useState(false);

  // Auto-save
  const [autoSave, setAutoSave] = useState<boolean>(readAutoSave);

  // Recent projects
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(readRecentProjects);

  // ---------------------------------------------------------------------------
  // Snapshot helpers
  // ---------------------------------------------------------------------------

  function currentSnapshot(): EntitySnapshot {
    return { meta, confederations, countries, teams, players, staff, names, competitions };
  }

  function applySnapshot(snapshot: EntitySnapshot) {
    setMeta(snapshot.meta);
    setConfederations(snapshot.confederations);
    setCountries(snapshot.countries);
    setTeams(snapshot.teams);
    setPlayers(snapshot.players);
    setStaff(snapshot.staff);
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
    youthEditor.syncEditing(snapshot.players);
    staffEditor.syncEditing(snapshot.staff);
    compEditor.syncEditing(snapshot.competitions);
    syncNamesPoolEditing(snapshot.names);
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

  function flashSuccess(msg: string) {
    setSuccessMsg(msg);
    setTimeout(() => setSuccessMsg(null), 5000);
  }

  function loadProjectState(data: PackageProjectData) {
    setMeta(data.meta);
    setConfederations(data.confederations);
    setCountries(data.countries);
    setTeams(data.teams);
    setPlayers(data.players);
    setStaff(data.staff ?? []);
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

  // Latest committed slices, read at write time so a serialized/queued persist
  // writes one consistent snapshot rather than whatever its closure captured.
  const stateRef = useRef({ meta, confederations, countries, teams, players, staff, names, competitions });
  stateRef.current = { meta, confederations, countries, teams, players, staff, names, competitions };

  // Serializes save_package_project writes so concurrent saves can't interleave
  // full-file writes or let an older write land after a newer one.
  const enqueueWrite = useRef(createWriteQueue()).current;

  const persist = useCallback((overrides?: {
    meta?: WorldMetaDef;
    confederations?: ConfederationDef[];
    countries?: CountryDef[];
    teams?: TeamDef[];
    players?: PlayerDef[];
    staff?: StaffDef[];
    names?: NamesDefinition;
    competitions?: CompetitionDef[];
  }) => enqueueWrite(async () => {
    const s = stateRef.current;
    setSaveState("saving");
    try {
      await invoke("save_package_project", {
        dir: projectDir,
        meta: overrides?.meta ?? s.meta,
        confederations: overrides?.confederations ?? s.confederations,
        countries: overrides?.countries ?? s.countries,
        teams: overrides?.teams ?? s.teams,
        players: overrides?.players ?? s.players,
        staff: overrides?.staff ?? s.staff,
        names: overrides?.names ?? s.names,
        competitions: overrides?.competitions ?? s.competitions,
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
  }), [projectDir]);

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
    setSectionFormPanels((prev) => ({ ...prev, [selectedSection]: formPanel }));
    setSelectedSection(section);
    if (section === "metadata") {
      setFormPanel("metadata");
    } else {
      setFormPanel(sectionFormPanels[section] ?? "empty");
    }
  }

  function handleShowIssues() {
    setFormPanel("issues");
  }

  // ---------------------------------------------------------------------------
  // Top-level project handlers
  // ---------------------------------------------------------------------------

  async function handleNewPackage(newMeta: WorldMetaDef, sample: SamplePackage | null) {
    setIsBusy(true);
    try {
      const dir = await invoke<string>("create_world_project", { slug: newMeta.id, meta: newMeta });
      if (sample) {
        await invoke("save_package_project", {
          dir,
          meta: newMeta,
          confederations: sample.confederations,
          countries: sample.countries,
          teams: sample.teams,
          players: sample.players,
          staff: sample.staff ?? [],
          names: sample.names,
          competitions: sample.competitions,
        });
      }
      const data = await invoke<PackageProjectData>("read_package_project", { dir });
      setProjectDir(dir);
      loadProjectState(data);
      addRecentProject(dir, newMeta.name || newMeta.id);
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
    let selected: string | string[] | null;
    try {
      selected = await open({
        directory: false,
        multiple: false,
        filters: [
          { name: "World Package", extensions: ["ofm"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });
    } catch {
      return;
    }
    if (typeof selected === "string") {
      await openFromPath(selected);
      return;
    }
    let dirFallback: string | string[] | null;
    try {
      dirFallback = await open({ directory: true, multiple: false });
    } catch {
      return;
    }
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
    let outPath: string | null;
    try {
      outPath = await save({
        filters: [{ name: "OFM Package", extensions: ["ofm"] }],
        defaultPath: defaultName,
      });
    } catch {
      return;
    }
    if (typeof outPath !== "string") return;
    setIsBusy(true);
    try {
      await persist();
    } catch {
      setIsBusy(false);
      return;
    }
    try {
      await invoke("build_ofm", { dir: projectDir, output: outPath });
      flashSuccess(t("worldEditor.buildSuccess"));
    } catch (err) {
      flashError(resolveBackendError(err));
    } finally {
      setIsBusy(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Entity editors
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

  const youthEditor = useEntityEditor({
    items: players,
    setItems: setPlayers,
    empty: () => ({ ...emptyPlayer(), youth: true }),
    captureHistory,
    saveItems: (items) => persist({ players: items }),
    autoSave,
    onOpen: () => setFormPanel("youth"),
    onClose: () => setFormPanel("empty"),
    setIsBusy,
  });

  const staffEditor = useEntityEditor({
    items: staff,
    setItems: setStaff,
    empty: emptyStaff,
    captureHistory,
    saveItems: (items) => persist({ staff: items }),
    autoSave,
    onOpen: () => setFormPanel("staff"),
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
  // Names pool
  // ---------------------------------------------------------------------------

  const {
    editingPoolKey,
    editingPool,
    isNewPool,
    revision: namesPoolRevision,
    handleSelectPool,
    handleAddPool,
    handleDeletePool,
    handleSavePool,
    syncEditing: syncNamesPoolEditing,
  } = useNamesPoolEditor({
    names,
    setNames,
    autoSave,
    captureHistory,
    saveNames: (n) => persist({ names: n }),
    onOpen: () => setFormPanel("names-pool"),
    onClose: () => setFormPanel("empty"),
    setIsBusy,
  });

  // ---------------------------------------------------------------------------
  // Home view (no project open)
  // ---------------------------------------------------------------------------

  if (!projectDir) {
    return (
      <WorldEditorHome
        isBusy={isBusy}
        errorMsg={errorMsg}
        recentProjects={recentProjects}
        onNewPackage={(m, sample) => { void handleNewPackage(m, sample); }}
        onOpenPackage={() => { void handleOpenPackage(); }}
        onOpenRecent={(path) => { void openFromPath(path); }}
      />
    );
  }

  // ---------------------------------------------------------------------------
  // 3-column editor layout
  // ---------------------------------------------------------------------------

  const listPanel =
    selectedSection === "metadata" ? null : (
      <WorldEditorListContent
        selectedSection={selectedSection}
        formPanel={formPanel}
        teams={teams}
        players={players}
        staff={staff}
        confederations={confederations}
        countries={countries}
        competitions={competitions}
        names={names}
        projectDir={projectDir || undefined}
        teamEditor={teamEditor}
        playerEditor={playerEditor}
        youthEditor={youthEditor}
        staffEditor={staffEditor}
        confEditor={confEditor}
        countryEditor={countryEditor}
        compEditor={compEditor}
        namesEditor={{ editingPoolKey, handleAddPool, handleSelectPool, handleDeletePool }}
      />
    );

  return (
    <>
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
          youthCount={players.filter((p) => p.youth).length}
          staffCount={staff.length}
          namePoolCount={Object.keys(names.pools).length}
          competitionCount={competitions.length}
          issueCount={issues.length}
          onShowIssues={handleShowIssues}
          showingIssues={formPanel === "issues"}
        />
      }
      listPanel={listPanel}
      formPanel={
        <WorldEditorFormPanel
          formPanel={formPanel}
          isBusy={isBusy}
          projectDir={projectDir}
          meta={meta}
          onMetaChange={(m) => { setMeta(m); setIsDirty(true); }}
          onSaveMetadata={() => { pushHistory(currentSnapshot()); void persist({ meta }).catch(() => {}); }}
          counts={{
            teams: teams.length,
            players: players.length,
            confederations: confederations.length,
            countries: countries.length,
            competitions: competitions.length,
            namePools: Object.keys(names.pools).length,
          }}
          issues={issues}
          teamEditor={teamEditor}
          confEditor={confEditor}
          countryEditor={countryEditor}
          playerEditor={playerEditor}
          youthEditor={youthEditor}
          staffEditor={staffEditor}
          compEditor={compEditor}
          confederations={confederations}
          teams={teams}
          editingPoolKey={editingPoolKey}
          editingPool={editingPool}
          isNewPool={isNewPool}
          namesPoolRevision={namesPoolRevision}
          poolKeys={Object.keys(names.pools)}
          onSavePool={(key, pool) => { void handleSavePool(key, pool); }}
          onBack={() => setFormPanel("empty")}
        />
      }
    />

    {/* Floating notifications */}
    {(errorMsg || successMsg) && (
      <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm pointer-events-none">
        {errorMsg && (
          <div className="flex items-center gap-2 rounded-xl border border-red-300 dark:border-red-500/40 bg-red-50 dark:bg-red-500/10 px-4 py-3 shadow-lg text-sm text-red-700 dark:text-red-300">
            <AlertCircle className="w-4 h-4 flex-shrink-0" />
            <span>{errorMsg}</span>
          </div>
        )}
        {successMsg && (
          <div className="flex items-center gap-2 rounded-xl border border-green-300 dark:border-green-500/40 bg-green-50 dark:bg-green-500/10 px-4 py-3 shadow-lg text-sm text-green-700 dark:text-green-300">
            <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
            <span>{successMsg}</span>
          </div>
        )}
      </div>
    )}
    </>
  );
}
