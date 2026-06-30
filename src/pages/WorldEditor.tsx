import { AlertCircle, CheckCircle2 } from "lucide-react";
import { WorldEditorHome } from "../components/worldEditor/WorldEditorHome";
import { WorldEditorLayout } from "../components/worldEditor/WorldEditorLayout";
import { WorldEditorTopBar } from "../components/worldEditor/WorldEditorTopBar";
import { WorldEditorSidebar } from "../components/worldEditor/WorldEditorSidebar";
import { WorldEditorFormPanel } from "../components/worldEditor/WorldEditorFormPanel";
import { WorldEditorListContent } from "../components/worldEditor/WorldEditorListContent";
import { useWorldEditor } from "./useWorldEditor";

export default function WorldEditor() {
  const {
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
    projectDir,
    selectedSection,
    formPanel,
    setFormPanel,
    handleSelectSection,
    handleShowIssues,
    isBusy,
    errorMsg,
    successMsg,
    saveState,
    isDirty,
    setIsDirty,
    autoSave,
    handleToggleAutoSave,
    recentProjects,
    canUndo,
    canRedo,
    handleUndo,
    handleRedo,
    pushHistory,
    currentSnapshot,
    persist,
    handleManualSave,
    handleNewPackage,
    openFromPath,
    handleOpenPackage,
    handleValidate,
    handleBuild,
    teamEditor,
    confEditor,
    countryEditor,
    playerEditor,
    youthEditor,
    staffEditor,
    compEditor,
    editingPoolKey,
    editingPool,
    isNewPool,
    namesPoolRevision,
    handleSelectPool,
    handleAddPool,
    handleDeletePool,
    handleSavePool,
  } = useWorldEditor();

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
