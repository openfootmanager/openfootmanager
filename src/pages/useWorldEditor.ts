import { useState, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
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
import { useFlashMessages } from "../hooks/useFlashMessages";
import { useRecentProjects } from "../hooks/useRecentProjects";
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
import type { SamplePackage } from "../components/menu/PackageEditor/sampleData";
import type { SaveState } from "../components/worldEditor/WorldEditorTopBar";
import type { FormPanel } from "../components/worldEditor/WorldEditorFormPanel";

const AUTO_SAVE_KEY = "worldEditor.autoSave";

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

export function useWorldEditor() {
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
  const { errorMsg, successMsg, flashError, flashSuccess } = useFlashMessages();
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [isDirty, setIsDirty] = useState(false);

  // Auto-save
  const [autoSave, setAutoSave] = useState<boolean>(readAutoSave);

  // Recent projects
  const { recentProjects, addRecentProject } = useRecentProjects();

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

  return {
    // Entity state
    meta,
    setMeta,
    confederations,
    countries,
    teams,
    players,
    staff,
    names,
    competitions,
    issues,
    // Layout state
    projectDir,
    selectedSection,
    formPanel,
    setFormPanel,
    handleSelectSection,
    handleShowIssues,
    // Async / status
    isBusy,
    errorMsg,
    successMsg,
    saveState,
    isDirty,
    setIsDirty,
    autoSave,
    handleToggleAutoSave,
    // Recent projects
    recentProjects,
    // Undo / redo + history
    canUndo,
    canRedo,
    handleUndo,
    handleRedo,
    pushHistory,
    currentSnapshot,
    // Persistence / lifecycle
    persist,
    handleManualSave,
    handleNewPackage,
    openFromPath,
    handleOpenPackage,
    handleValidate,
    handleBuild,
    // Entity editors
    teamEditor,
    confEditor,
    countryEditor,
    playerEditor,
    youthEditor,
    staffEditor,
    compEditor,
    // Names pool
    editingPoolKey,
    editingPool,
    isNewPool,
    namesPoolRevision,
    handleSelectPool,
    handleAddPool,
    handleDeletePool,
    handleSavePool,
  };
}
