import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { resolveBackendError } from "../../../utils/backendI18n";
import {
  emptyCompetition,
  emptyConfederation,
  emptyCountry,
  emptyMeta,
  emptyNamesDefinition,
  emptyPlayer,
  emptyTeam,
} from "./helpers";
import { HomeView } from "./HomeView";
import { EditView } from "./EditView";
import { TeamForm } from "./TeamForm";
import { ConfederationForm } from "./ConfederationForm";
import { CountryForm } from "./CountryForm";
import { PlayerForm } from "./PlayerForm";
import { NamesPoolForm } from "./NamesPoolForm";
import { CompetitionForm } from "./CompetitionForm";
import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  EditTab,
  EditorView,
  NamePool,
  NamesDefinition,
  PackageProjectData,
  PlayerDef,
  TeamDef,
  WorldMetaDef,
} from "./types";

interface PackageEditorProps {
  onBack: () => void;
}

export default function PackageEditor({ onBack }: PackageEditorProps) {
  const { t } = useTranslation();

  const [view, setView] = useState<EditorView>("home");
  const [tab, setTab] = useState<EditTab>("metadata");
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

  // UI state
  const [isBusy, setIsBusy] = useState(false);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Per-entity editing state
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
  // Helpers
  // ---------------------------------------------------------------------------

  function flashSuccess(msg: string) {
    setSuccessMsg(msg);
    setErrorMsg(null);
    setTimeout(() => setSuccessMsg(null), 3000);
  }

  function flashError(msg: string) {
    setErrorMsg(msg);
    setSuccessMsg(null);
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
  }

  function resetProjectState() {
    setMeta(emptyMeta());
    setConfederations([]);
    setCountries([]);
    setTeams([]);
    setPlayers([]);
    setNames(emptyNamesDefinition());
    setCompetitions([]);
    setIssues([]);
  }

  async function persist(overrides?: {
    meta?: WorldMetaDef;
    confederations?: ConfederationDef[];
    countries?: CountryDef[];
    teams?: TeamDef[];
    players?: PlayerDef[];
    names?: NamesDefinition;
    competitions?: CompetitionDef[];
  }) {
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
  }

  // ---------------------------------------------------------------------------
  // Top-level handlers
  // ---------------------------------------------------------------------------

  async function handleNewPackage() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;
    const newMeta = emptyMeta();
    setIsBusy(true);
    try {
      await invoke("create_package_project", { dir, meta: newMeta });
      setProjectDir(dir);
      resetProjectState();
      setMeta(newMeta);
      setTab("metadata");
      setView("edit");
    } catch (err) {
      flashError(resolveBackendError(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function handleOpenPackage() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;
    setIsBusy(true);
    try {
      const data = await invoke<PackageProjectData>("read_package_project", { dir });
      setProjectDir(dir);
      loadProjectState(data);
      setTab("metadata");
      setView("edit");
    } catch (err) {
      flashError(resolveBackendError(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function handleSave() {
    setIsBusy(true);
    try {
      await persist();
      flashSuccess(t("packageEditor.saveSuccess"));
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
      if (data.issues.length === 0) {
        flashSuccess(t("packageEditor.validate") + " ✓");
      }
    } catch (err) {
      flashError(resolveBackendError(err));
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
      flashSuccess(t("packageEditor.buildSuccess"));
    } catch (err) {
      flashError(resolveBackendError(err));
    } finally {
      setIsBusy(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Team handlers
  // ---------------------------------------------------------------------------

  function handleAddTeam() {
    setEditingTeam(emptyTeam());
    setEditingTeamIndex(null);
    setView("team");
  }

  function handleEditTeam(index: number) {
    setEditingTeam({ ...teams[index] });
    setEditingTeamIndex(index);
    setView("team");
  }

  function handleDeleteTeam(index: number) {
    setTeams((prev) => prev.filter((_, i) => i !== index));
  }

  async function handleSaveTeam() {
    const updated =
      editingTeamIndex === null
        ? [...teams, editingTeam]
        : teams.map((t, i) => (i === editingTeamIndex ? editingTeam : t));
    setTeams(updated);
    setIsBusy(true);
    try {
      await persist({ teams: updated });
    } catch {
      // Non-fatal — changes are in local state; user can retry via Save
    } finally {
      setIsBusy(false);
    }
    setTab("teams");
    setView("edit");
  }

  function updateTeamField<K extends keyof TeamDef>(key: K, value: TeamDef[K]) {
    setEditingTeam((prev) => ({ ...prev, [key]: value }));
  }

  // ---------------------------------------------------------------------------
  // Confederation handlers
  // ---------------------------------------------------------------------------

  function handleAddConfederation() {
    setEditingConf(emptyConfederation());
    setEditingConfIndex(null);
    setView("confederation");
  }

  function handleEditConfederation(index: number) {
    setEditingConf({ ...confederations[index] });
    setEditingConfIndex(index);
    setView("confederation");
  }

  function handleDeleteConfederation(index: number) {
    setConfederations((prev) => prev.filter((_, i) => i !== index));
  }

  async function handleSaveConfederation() {
    const updated =
      editingConfIndex === null
        ? [...confederations, editingConf]
        : confederations.map((c, i) => (i === editingConfIndex ? editingConf : c));
    setConfederations(updated);
    setIsBusy(true);
    try {
      await persist({ confederations: updated });
    } catch {
      // Non-fatal
    } finally {
      setIsBusy(false);
    }
    setTab("confederations");
    setView("edit");
  }

  function updateConfField<K extends keyof ConfederationDef>(key: K, value: ConfederationDef[K]) {
    setEditingConf((prev) => ({ ...prev, [key]: value }));
  }

  // ---------------------------------------------------------------------------
  // Country handlers
  // ---------------------------------------------------------------------------

  function handleAddCountry() {
    setEditingCountry(emptyCountry());
    setEditingCountryIndex(null);
    setView("country");
  }

  function handleEditCountry(index: number) {
    setEditingCountry({ ...countries[index] });
    setEditingCountryIndex(index);
    setView("country");
  }

  function handleDeleteCountry(index: number) {
    setCountries((prev) => prev.filter((_, i) => i !== index));
  }

  async function handleSaveCountry() {
    const updated =
      editingCountryIndex === null
        ? [...countries, editingCountry]
        : countries.map((c, i) => (i === editingCountryIndex ? editingCountry : c));
    setCountries(updated);
    setIsBusy(true);
    try {
      await persist({ countries: updated });
    } catch {
      // Non-fatal
    } finally {
      setIsBusy(false);
    }
    setTab("countries");
    setView("edit");
  }

  function updateCountryField<K extends keyof CountryDef>(key: K, value: CountryDef[K]) {
    setEditingCountry((prev) => ({ ...prev, [key]: value }));
  }

  // ---------------------------------------------------------------------------
  // Player handlers
  // ---------------------------------------------------------------------------

  function handleAddPlayer() {
    setEditingPlayer(emptyPlayer());
    setEditingPlayerIndex(null);
    setView("player");
  }

  function handleEditPlayer(index: number) {
    setEditingPlayer({ ...players[index] });
    setEditingPlayerIndex(index);
    setView("player");
  }

  function handleDeletePlayer(index: number) {
    setPlayers((prev) => prev.filter((_, i) => i !== index));
  }

  async function handleSavePlayer() {
    const updated =
      editingPlayerIndex === null
        ? [...players, editingPlayer]
        : players.map((p, i) => (i === editingPlayerIndex ? editingPlayer : p));
    setPlayers(updated);
    setIsBusy(true);
    try {
      await persist({ players: updated });
    } catch {
      // Non-fatal
    } finally {
      setIsBusy(false);
    }
    setTab("players");
    setView("edit");
  }

  function updatePlayerField<K extends keyof PlayerDef>(key: K, value: PlayerDef[K]) {
    setEditingPlayer((prev) => ({ ...prev, [key]: value }));
  }

  // ---------------------------------------------------------------------------
  // Names pool handlers
  // ---------------------------------------------------------------------------

  function handleAddPool() {
    setEditingPoolKey("");
    setEditingPool({ first_names: [], last_names: [] });
    setIsNewPool(true);
    setView("names-pool");
  }

  function handleEditPool(key: string) {
    setEditingPoolKey(key);
    setEditingPool({ ...names.pools[key] });
    setIsNewPool(false);
    setView("names-pool");
  }

  function handleDeletePool(key: string) {
    const updated: NamesDefinition = {
      ...names,
      pools: Object.fromEntries(Object.entries(names.pools).filter(([k]) => k !== key)),
    };
    setNames(updated);
  }

  async function handleSavePool(key: string, pool: NamePool) {
    const updatedPools = isNewPool
      ? { ...names.pools, [key]: pool }
      : Object.fromEntries(
          Object.entries(names.pools).map(([k, v]) =>
            k === editingPoolKey ? [key, pool] : [k, v],
          ),
        );
    const updated: NamesDefinition = { ...names, pools: updatedPools };
    setNames(updated);
    setIsBusy(true);
    try {
      await persist({ names: updated });
    } catch {
      // Non-fatal
    } finally {
      setIsBusy(false);
    }
    setTab("names");
    setView("edit");
  }

  // ---------------------------------------------------------------------------
  // Competition handlers
  // ---------------------------------------------------------------------------

  function handleAddCompetition() {
    setEditingComp(emptyCompetition());
    setEditingCompIndex(null);
    setView("competition");
  }

  function handleEditCompetition(index: number) {
    setEditingComp({ ...competitions[index] });
    setEditingCompIndex(index);
    setView("competition");
  }

  function handleDeleteCompetition(index: number) {
    setCompetitions((prev) => prev.filter((_, i) => i !== index));
  }

  async function handleSaveCompetition() {
    const updated =
      editingCompIndex === null
        ? [...competitions, editingComp]
        : competitions.map((c, i) => (i === editingCompIndex ? editingComp : c));
    setCompetitions(updated);
    setIsBusy(true);
    try {
      await persist({ competitions: updated });
    } catch {
      // Non-fatal
    } finally {
      setIsBusy(false);
    }
    setTab("competitions");
    setView("edit");
  }

  function updateCompField<K extends keyof CompetitionDef>(key: K, value: CompetitionDef[K]) {
    setEditingComp((prev) => ({ ...prev, [key]: value }));
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (view === "home") {
    return (
      <HomeView
        isBusy={isBusy}
        errorMsg={errorMsg}
        onBack={onBack}
        onNewPackage={() => { void handleNewPackage(); }}
        onOpenPackage={() => { void handleOpenPackage(); }}
      />
    );
  }

  if (view === "team") {
    return (
      <TeamForm
        editingTeam={editingTeam}
        editingTeamIndex={editingTeamIndex}
        isBusy={isBusy}
        onBack={() => { setTab("teams"); setView("edit"); }}
        onSave={() => { void handleSaveTeam(); }}
        updateField={updateTeamField}
      />
    );
  }

  if (view === "confederation") {
    return (
      <ConfederationForm
        editing={editingConf}
        editingIndex={editingConfIndex}
        isBusy={isBusy}
        onBack={() => { setTab("confederations"); setView("edit"); }}
        onSave={() => { void handleSaveConfederation(); }}
        updateField={updateConfField}
      />
    );
  }

  if (view === "country") {
    return (
      <CountryForm
        editing={editingCountry}
        editingIndex={editingCountryIndex}
        confederations={confederations}
        isBusy={isBusy}
        onBack={() => { setTab("countries"); setView("edit"); }}
        onSave={() => { void handleSaveCountry(); }}
        updateField={updateCountryField}
      />
    );
  }

  if (view === "player") {
    return (
      <PlayerForm
        editing={editingPlayer}
        editingIndex={editingPlayerIndex}
        isBusy={isBusy}
        onBack={() => { setTab("players"); setView("edit"); }}
        onSave={() => { void handleSavePlayer(); }}
        updateField={updatePlayerField}
      />
    );
  }

  if (view === "names-pool") {
    return (
      <NamesPoolForm
        poolKey={editingPoolKey}
        pool={editingPool}
        isNew={isNewPool}
        isBusy={isBusy}
        onBack={() => { setTab("names"); setView("edit"); }}
        onSave={(key, pool) => { void handleSavePool(key, pool); }}
      />
    );
  }

  if (view === "competition") {
    return (
      <CompetitionForm
        editing={editingComp}
        editingIndex={editingCompIndex}
        isBusy={isBusy}
        onBack={() => { setTab("competitions"); setView("edit"); }}
        onSave={() => { void handleSaveCompetition(); }}
        updateField={updateCompField}
      />
    );
  }

  return (
    <EditView
      tab={tab}
      meta={meta}
      confederations={confederations}
      countries={countries}
      teams={teams}
      players={players}
      names={names}
      competitions={competitions}
      issues={issues}
      isBusy={isBusy}
      successMsg={successMsg}
      errorMsg={errorMsg}
      onBack={() => setView("home")}
      onTabChange={setTab}
      onSave={() => { void handleSave(); }}
      onValidate={() => { void handleValidate(); }}
      onBuild={() => { void handleBuild(); }}
      onMetaChange={setMeta}
      onAddConfederation={handleAddConfederation}
      onEditConfederation={handleEditConfederation}
      onDeleteConfederation={handleDeleteConfederation}
      onAddCountry={handleAddCountry}
      onEditCountry={handleEditCountry}
      onDeleteCountry={handleDeleteCountry}
      onAddTeam={handleAddTeam}
      onEditTeam={handleEditTeam}
      onDeleteTeam={handleDeleteTeam}
      onAddPlayer={handleAddPlayer}
      onEditPlayer={handleEditPlayer}
      onDeletePlayer={handleDeletePlayer}
      onAddPool={handleAddPool}
      onEditPool={handleEditPool}
      onDeletePool={handleDeletePool}
      onAddCompetition={handleAddCompetition}
      onEditCompetition={handleEditCompetition}
      onDeleteCompetition={handleDeleteCompetition}
    />
  );
}
