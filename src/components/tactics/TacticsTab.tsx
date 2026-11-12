import type { JSX } from "react";
import { useCallback, useState } from "react";
import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import type { PlayerRole } from "../../store/types";
import { useTranslation } from "react-i18next";
import { setPlayerRole } from "../../services/squadService";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";

import TacticsLineupSection from "./TacticsLineupSection";
import { effectivePlayerRoles } from "./TacticsTab.helpers";
import TacticsSetPieces from "./TacticsSetPieces";
import TacticsWorkbench from "./TacticsWorkbench";
import TacticsCommandBar from "./TacticsCommandBar";
import TacticsCompareDialog from "./TacticsCompareDialog";
import TacticsPresetDialog from "./TacticsPresetDialog";
import { usePresetChoice } from "./usePresetChoice";
import { useTacticsLibrary } from "./useTacticsLibrary";
import { useTacticsLineup } from "./useTacticsLineup";

interface TacticsTabProps {
  gameState: GameStateData | null;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate: (g: GameStateData) => void;
}

export default function TacticsTab({
  gameState,
  onSelectPlayer,
  onGameUpdate: onGameUpdateProp,
}: TacticsTabProps): JSX.Element {
  const { t } = useTranslation();
  // Why the last tactic write did not save, in the player's language. A write that succeeds
  // clears it: the screen then shows what the backend holds.
  const [writeError, setWriteError] = useState<string | null>(null);
  const onWriteError = useCallback((error: unknown) => {
    setWriteError(resolveTranslatedErrorMessage(error));
  }, []);
  const onGameUpdate = useCallback(
    (game: GameStateData) => {
      setWriteError(null);
      onGameUpdateProp(game);
    },
    [onGameUpdateProp],
  );

  const lineupState = useTacticsLineup({ gameState, onGameUpdate, onWriteError });
  const {
    team,
    formation,
    activePlayStyle,
    initialPreset,
    roster,
    startingXI,
    autoSetPieces,
    selectedPlayer,
    comparePlayer,
    canConfirmSwap,
    dragPreviewRef,
    handleFormationChange,
    handlePlayStyleChange,
    handleApplyTacticPreset,
    clearLineupSelection,
    handleConfirmSwap,
    handleTacticsPhaseChange,
    lineupReady,
    lineup,
  } = lineupState;

  // The roles shown in the table and on the pitch: the match's own (the lineup's), not a stored one
  // a move to another slot left stale.
  const effectiveRoles = effectivePlayerRoles(lineup, team?.player_roles);

  // A role chosen in the table or on the pitch: one write, and a refusal is shown, not only logged.
  function changePlayerRole(playerId: string, role: PlayerRole) {
    void setPlayerRole(playerId, role)
      .then(onGameUpdate)
      .catch((error: unknown) => {
        console.error("Failed to set player role:", error);
        onWriteError(error);
      });
  }

  const {
    activeTactic,
    tacticLibrary,
    isCommandBarDirty,
    libraryLoaded,
    canResetTactic,
    handleResetTactic,
    applyTacticSelection,
    handleCreateCustomTactic,
    handleDuplicateTactic,
    handleSaveTactic,
    canDeleteTactic,
    handleDeleteTactic,
  } = useTacticsLibrary({
    gameState,
    formation,
    activePlayStyle,
    initialPreset,
    onApplyPreset: handleApplyTacticPreset,
    onKeepInstructions: (nextFormation, playStyle) =>
      handleApplyTacticPreset(nextFormation, playStyle, { keepInstructions: true }),
    onGameUpdate,
    onWriteError,
    onWriteSaved: () => setWriteError(null),
  });

  const presetChoice = usePresetChoice((tactic, keepInstructions) =>
    applyTacticSelection(tactic, { keepInstructions }),
  );

  if (!team) {
    return <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>;
  }

  return (
    // The tab fills the workspace (it is one of `tabFillsWorkspace`'s): its regions scroll, the page
    // doesn't.
    <div className="flex min-h-0 w-full flex-1 flex-col gap-4">
      <div
        ref={dragPreviewRef}
        aria-hidden="true"
        className="pointer-events-none fixed -left-20 top-0 h-8 w-8 rounded-full border border-white/15 bg-navy-900/90 shadow-lg"
      />
      {writeError ? (
        <p
          role="alert"
          className="rounded-lg border border-line bg-danger-tint px-4 py-2 text-body font-medium text-danger"
        >
          {writeError}
        </p>
      ) : null}
      <TacticsCommandBar
        activeTactic={activeTactic}
        activePlayStyle={activePlayStyle}
        formation={formation}
        isDirty={isCommandBarDirty}
        libraryLoaded={libraryLoaded}
        canReset={canResetTactic}
        onReset={() => {
          void handleResetTactic();
        }}
        canDelete={canDeleteTactic}
        onCreateNew={handleCreateCustomTactic}
        onDelete={handleDeleteTactic}
        onDuplicate={handleDuplicateTactic}
        onFormationChange={(nextFormation) => {
          void handleFormationChange(nextFormation);
        }}
        onPlayStyleChange={(playStyle) => {
          void handlePlayStyleChange(playStyle);
        }}
        onSave={handleSaveTactic}
        onSelectTactic={(id) => {
          const nextTactic = tacticLibrary.find((entry) => entry.id === id);
          if (!nextTactic) {
            return;
          }

          void presetChoice.choose(nextTactic);
        }}
        tacticLibrary={tacticLibrary}
      />

      {lineupReady ? null : (
        <p role="status" className="text-caption text-fg-muted">
          {t("common.lineupLoading")}
        </p>
      )}

      <TacticsWorkbench
        tacticsPhase={team?.tactics_phase}
        onTacticsPhaseChange={(patch) => {
          void handleTacticsPhaseChange(patch);
        }}
        setPieces={
          <TacticsSetPieces
            allSquad={roster}
            autoSetPieces={autoSetPieces}
            matchRoles={team.match_roles}
            onGameUpdate={onGameUpdate}
            onWriteError={onWriteError}
            startingPlayers={startingXI}
          />
        }
      >
        <TacticsLineupSection
          state={lineupState}
          playerRoles={effectiveRoles}
          onRoleChange={changePlayerRole}
          onSelectPlayer={onSelectPlayer}
        />
      </TacticsWorkbench>

      {presetChoice.pending ? (
        <TacticsPresetDialog
          name={presetChoice.pending.name}
          onReplace={() => void presetChoice.replaceInstructions()}
          onKeep={() => void presetChoice.keepInstructions()}
          onCancel={presetChoice.cancel}
        />
      ) : null}

      {/* Inspector modal — only when both players are selected for comparison */}
      {selectedPlayer && comparePlayer && (
        <TacticsCompareDialog
          canConfirmSwap={canConfirmSwap}
          comparePlayer={comparePlayer}
          onClose={clearLineupSelection}
          onConfirmSwap={() => {
            void handleConfirmSwap();
          }}
          selectedPlayer={selectedPlayer}
        />
      )}
    </div>
  );
}
