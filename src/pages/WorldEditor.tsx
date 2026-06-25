import { useState, useCallback, useEffect, useRef } from "react";
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
import { WorldEditorHome } from "../components/worldEditor/WorldEditorHome";
import { WorldEditorLayout } from "../components/worldEditor/WorldEditorLayout";
import { WorldEditorTopBar, type SaveState } from "../components/worldEditor/WorldEditorTopBar";
import { WorldEditorSidebar } from "../components/worldEditor/WorldEditorSidebar";
import { EntityListPanel } from "../components/worldEditor/EntityListPanel";

const AUTO_SAVE_KEY = "worldEditor.autoSave";
const MAX_HISTORY = 50;

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

  // Async state
  const [isBusy, setIsBusy] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [isDirty, setIsDirty] = useState(false);

  // Auto-save
  const [autoSave, setAutoSave] = useState<boolean>(readAutoSave);

  // Undo/redo history (entity data only, not layout)
  const undoStack = useRef<EntitySnapshot[]>([]);
  const redoStack = useRef<EntitySnapshot[]>([]);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);

  // Per-entity editing buffers
  const [editingTeam, setEditingTeam] = useState<TeamDef>(emptyTeam());
  const [editingTeamIndex, setEditingTeamIndex] = useState<number | null>(null);

  const [editingConf, setEditingConf] = useState<ConfederationDef>(emptyConfederation());
  const [editingConfIndex, setEditingConfIndex] = useState<number | null>(null);

  const [editingCountry, setEditingCountry] = useState<CountryDef>(emptyCountry());
  const [editingCountryIndex, setEditingCountryIndex] = useState<number | null>(null);

  const [editingPlayer, setEditingPlayer] = useState<PlayerDef>(emptyPlayer());
  const [editingPlayerIndex, setEditingPlayerIndex] = useState<number | null>(null);

  const [editingPoolKey, setEditingPoolKey] = useState("");
  const [editingPool, setEditingPool] = useState<NamePool>({ first_names: [], last_names: [] });
  const [isNewPool, setIsNewPool] = useState(false);

  const [editingComp, setEditingComp] = useState<CompetitionDef>(emptyCompetition());
  const [editingCompIndex, setEditingCompIndex] = useState<number | null>(null);

  // ---------------------------------------------------------------------------
  // History helpers
  // ---------------------------------------------------------------------------

  function currentSnapshot(): EntitySnapshot {
    return { meta, confederations, countries, teams, players, names, competitions };
  }

  function pushHistory(snapshot: EntitySnapshot) {
    undoStack.current = [...undoStack.current.slice(-MAX_HISTORY + 1), snapshot];
    redoStack.current = [];
    setCanUndo(true);
    setCanRedo(false);
    setIsDirty(true);
  }

  function applySnapshot(snapshot: EntitySnapshot) {
    setMeta(snapshot.meta);
    setConfederations(snapshot.confederations);
    setCountries(snapshot.countries);
    setTeams(snapshot.teams);
    setPlayers(snapshot.players);
    setNames(snapshot.names);
    setCompetitions(snapshot.competitions);
    setFormPanel("empty");
    setIsDirty(true);
  }

  function handleUndo() {
    if (undoStack.current.length === 0) return;
    const prev = undoStack.current[undoStack.current.length - 1];
    undoStack.current = undoStack.current.slice(0, -1);
    redoStack.current = [currentSnapshot(), ...redoStack.current];
    applySnapshot(prev);
    setCanUndo(undoStack.current.length > 0);
    setCanRedo(true);
  }

  function handleRedo() {
    if (redoStack.current.length === 0) return;
    const next = redoStack.current[0];
    redoStack.current = redoStack.current.slice(1);
    undoStack.current = [...undoStack.current, currentSnapshot()];
    applySnapshot(next);
    setCanUndo(true);
    setCanRedo(redoStack.current.length > 0);
  }

  // Keyboard shortcuts: Ctrl+Z / Ctrl+Shift+Z — blocked when an input/textarea is focused
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (!projectDir) return;
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        handleUndo();
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "y" || (e.key === "z" && e.shiftKey))) {
        e.preventDefault();
        handleRedo();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

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
    undoStack.current = [];
    redoStack.current = [];
    setCanUndo(false);
    setCanRedo(false);
    setIsDirty(false);
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
    setSelectedSection(section);
    if (section === "metadata") {
      setFormPanel("metadata");
    } else {
      setFormPanel("empty");
    }
  }

  function handleShowIssues() {
    setFormPanel("issues");
  }

  // ---------------------------------------------------------------------------
  // Top-level project handlers
  // ---------------------------------------------------------------------------

  async function handleNewPackage() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;
    setIsBusy(true);
    try {
      const newMeta = emptyMeta();
      await invoke("create_package_project", { dir, meta: newMeta });
      setProjectDir(dir);
      loadProjectState({ meta: newMeta, confederations: [], countries: [], teams: [], players: [], names: null, competitions: [], issues: [] });
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
    // If user selected an .ofm file, extract it to a temp editing dir first
    let dir: string;
    if (typeof selected === "string" && selected.endsWith(".ofm")) {
      setIsBusy(true);
      try {
        dir = await invoke<string>("extract_ofm_for_editing", { ofmPath: selected });
      } catch (err) {
        flashError(resolveBackendError(err));
        setIsBusy(false);
        return;
      }
    } else if (typeof selected === "string") {
      dir = selected;
      setIsBusy(true);
    } else {
      // User may have cancelled; fall back to directory picker
      const dirFallback = await open({ directory: true, multiple: false });
      if (typeof dirFallback !== "string") return;
      dir = dirFallback;
      setIsBusy(true);
    }
    try {
      const data = await invoke<PackageProjectData>("read_package_project", { dir });
      setProjectDir(dir);
      loadProjectState(data);
      setSelectedSection("metadata");
      setFormPanel("metadata");
    } catch (err) {
      flashError(resolveBackendError(err));
    } finally {
      setIsBusy(false);
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
  // Team handlers
  // ---------------------------------------------------------------------------

  function handleSelectTeam(index: number) {
    setEditingTeam({ ...teams[index] });
    setEditingTeamIndex(index);
    setFormPanel("team");
  }

  function handleAddTeam() {
    setEditingTeam(emptyTeam());
    setEditingTeamIndex(null);
    setFormPanel("team");
  }

  function handleDeleteTeam(index: number) {
    pushHistory(currentSnapshot());
    const updated = teams.filter((_, i) => i !== index);
    setTeams(updated);
    if (autoSave) void persist({ teams: updated });
    if (editingTeamIndex === index) setFormPanel("empty");
  }

  async function handleSaveTeam() {
    pushHistory(currentSnapshot());
    const updated =
      editingTeamIndex === null
        ? [...teams, editingTeam]
        : teams.map((t, i) => (i === editingTeamIndex ? editingTeam : t));
    const newIndex = editingTeamIndex ?? updated.length - 1;
    setTeams(updated);
    setEditingTeamIndex(newIndex);
    if (autoSave) {
      setIsBusy(true);
      try {
        await persist({ teams: updated });
      } catch {
        // non-fatal
      } finally {
        setIsBusy(false);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Confederation handlers
  // ---------------------------------------------------------------------------

  function handleSelectConfederation(index: number) {
    setEditingConf({ ...confederations[index] });
    setEditingConfIndex(index);
    setFormPanel("confederation");
  }

  function handleAddConfederation() {
    setEditingConf(emptyConfederation());
    setEditingConfIndex(null);
    setFormPanel("confederation");
  }

  function handleDeleteConfederation(index: number) {
    pushHistory(currentSnapshot());
    const updated = confederations.filter((_, i) => i !== index);
    setConfederations(updated);
    if (autoSave) void persist({ confederations: updated });
    if (editingConfIndex === index) setFormPanel("empty");
  }

  async function handleSaveConfederation() {
    pushHistory(currentSnapshot());
    const updated =
      editingConfIndex === null
        ? [...confederations, editingConf]
        : confederations.map((c, i) => (i === editingConfIndex ? editingConf : c));
    const newIndex = editingConfIndex ?? updated.length - 1;
    setConfederations(updated);
    setEditingConfIndex(newIndex);
    if (autoSave) {
      setIsBusy(true);
      try {
        await persist({ confederations: updated });
      } catch {
        // non-fatal
      } finally {
        setIsBusy(false);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Country handlers
  // ---------------------------------------------------------------------------

  function handleSelectCountry(index: number) {
    setEditingCountry({ ...countries[index] });
    setEditingCountryIndex(index);
    setFormPanel("country");
  }

  function handleAddCountry() {
    setEditingCountry(emptyCountry());
    setEditingCountryIndex(null);
    setFormPanel("country");
  }

  function handleDeleteCountry(index: number) {
    pushHistory(currentSnapshot());
    const updated = countries.filter((_, i) => i !== index);
    setCountries(updated);
    if (autoSave) void persist({ countries: updated });
    if (editingCountryIndex === index) setFormPanel("empty");
  }

  async function handleSaveCountry() {
    pushHistory(currentSnapshot());
    const updated =
      editingCountryIndex === null
        ? [...countries, editingCountry]
        : countries.map((c, i) => (i === editingCountryIndex ? editingCountry : c));
    const newIndex = editingCountryIndex ?? updated.length - 1;
    setCountries(updated);
    setEditingCountryIndex(newIndex);
    if (autoSave) {
      setIsBusy(true);
      try {
        await persist({ countries: updated });
      } catch {
        // non-fatal
      } finally {
        setIsBusy(false);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Player handlers
  // ---------------------------------------------------------------------------

  function handleSelectPlayer(index: number) {
    setEditingPlayer({ ...players[index] });
    setEditingPlayerIndex(index);
    setFormPanel("player");
  }

  function handleAddPlayer() {
    setEditingPlayer(emptyPlayer());
    setEditingPlayerIndex(null);
    setFormPanel("player");
  }

  function handleDeletePlayer(index: number) {
    pushHistory(currentSnapshot());
    const updated = players.filter((_, i) => i !== index);
    setPlayers(updated);
    if (autoSave) void persist({ players: updated });
    if (editingPlayerIndex === index) setFormPanel("empty");
  }

  async function handleSavePlayer() {
    pushHistory(currentSnapshot());
    const updated =
      editingPlayerIndex === null
        ? [...players, editingPlayer]
        : players.map((p, i) => (i === editingPlayerIndex ? editingPlayer : p));
    const newIndex = editingPlayerIndex ?? updated.length - 1;
    setPlayers(updated);
    setEditingPlayerIndex(newIndex);
    if (autoSave) {
      setIsBusy(true);
      try {
        await persist({ players: updated });
      } catch {
        // non-fatal
      } finally {
        setIsBusy(false);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Names pool handlers
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
  // Competition handlers
  // ---------------------------------------------------------------------------

  function handleSelectCompetition(index: number) {
    setEditingComp({ ...competitions[index] });
    setEditingCompIndex(index);
    setFormPanel("competition");
  }

  function handleAddCompetition() {
    setEditingComp(emptyCompetition());
    setEditingCompIndex(null);
    setFormPanel("competition");
  }

  function handleDeleteCompetition(index: number) {
    pushHistory(currentSnapshot());
    const updated = competitions.filter((_, i) => i !== index);
    setCompetitions(updated);
    if (autoSave) void persist({ competitions: updated });
    if (editingCompIndex === index) setFormPanel("empty");
  }

  async function handleSaveCompetition() {
    pushHistory(currentSnapshot());
    const updated =
      editingCompIndex === null
        ? [...competitions, editingComp]
        : competitions.map((c, i) => (i === editingCompIndex ? editingComp : c));
    const newIndex = editingCompIndex ?? updated.length - 1;
    setCompetitions(updated);
    setEditingCompIndex(newIndex);
    if (autoSave) {
      setIsBusy(true);
      try {
        await persist({ competitions: updated });
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
        onNewPackage={() => { void handleNewPackage(); }}
        onOpenPackage={() => { void handleOpenPackage(); }}
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
            onAdd={handleAddTeam}
            onEdit={handleSelectTeam}
            onDelete={handleDeleteTeam}
            selectedIndex={formPanel === "team" ? editingTeamIndex : null}
            onSelect={handleSelectTeam}
          />
        )}
        {selectedSection === "players" && (
          <PlayersTab
            players={players}
            onAdd={handleAddPlayer}
            onEdit={handleSelectPlayer}
            onDelete={handleDeletePlayer}
            selectedIndex={formPanel === "player" ? editingPlayerIndex : null}
            onSelect={handleSelectPlayer}
          />
        )}
        {selectedSection === "confederations" && (
          <ConfederationsTab
            confederations={confederations}
            onAdd={handleAddConfederation}
            onEdit={handleSelectConfederation}
            onDelete={handleDeleteConfederation}
            selectedIndex={formPanel === "confederation" ? editingConfIndex : null}
            onSelect={handleSelectConfederation}
          />
        )}
        {selectedSection === "countries" && (
          <CountriesTab
            countries={countries}
            onAdd={handleAddCountry}
            onEdit={handleSelectCountry}
            onDelete={handleDeleteCountry}
            selectedIndex={formPanel === "country" ? editingCountryIndex : null}
            onSelect={handleSelectCountry}
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
            onAdd={handleAddCompetition}
            onEdit={handleSelectCompetition}
            onDelete={handleDeleteCompetition}
            selectedIndex={formPanel === "competition" ? editingCompIndex : null}
            onSelect={handleSelectCompetition}
          />
        )}
      </EntityListPanel>
    );

  // Col 3: form or empty state
  const formContent = (() => {
    if (formPanel === "metadata") {
      return (
        <div className="max-w-2xl">
          <h2 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white mb-5">
            {t("worldEditor.metadata")}
          </h2>
          <MetadataForm meta={meta} onChange={(m) => { setMeta(m); setIsDirty(true); }} />
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
            editingTeam={editingTeam}
            editingTeamIndex={editingTeamIndex}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void handleSaveTeam(); }}
            updateField={(key, value) => setEditingTeam((prev) => ({ ...prev, [key]: value }))}
          />
        </div>
      );
    }

    if (formPanel === "confederation") {
      return (
        <div className="max-w-lg">
          <ConfederationForm
            editing={editingConf}
            editingIndex={editingConfIndex}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void handleSaveConfederation(); }}
            updateField={(key, value) => setEditingConf((prev) => ({ ...prev, [key]: value }))}
          />
        </div>
      );
    }

    if (formPanel === "country") {
      return (
        <div className="max-w-lg">
          <CountryForm
            editing={editingCountry}
            editingIndex={editingCountryIndex}
            confederations={confederations}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void handleSaveCountry(); }}
            updateField={(key, value) => setEditingCountry((prev) => ({ ...prev, [key]: value }))}
          />
        </div>
      );
    }

    if (formPanel === "player") {
      return (
        <div className="max-w-lg">
          <PlayerForm
            editing={editingPlayer}
            editingIndex={editingPlayerIndex}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void handleSavePlayer(); }}
            updateField={(key, value) => setEditingPlayer((prev) => ({ ...prev, [key]: value }))}
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
            editing={editingComp}
            editingIndex={editingCompIndex}
            isBusy={isBusy}
            onBack={() => setFormPanel("empty")}
            onSave={() => { void handleSaveCompetition(); }}
            updateField={(key, value) => setEditingComp((prev) => ({ ...prev, [key]: value }))}
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
