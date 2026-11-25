import type { JSX } from "react";

import type { PlayerData } from "../../store/gameStore";
import TacticsCompareDialog from "./TacticsCompareDialog";
import TacticsPresetDialog from "./TacticsPresetDialog";
import type { usePresetChoice } from "./usePresetChoice";

interface TacticsDialogsProps {
  /** The question a preset asks before it replaces the manager's own instructions. */
  presetChoice: Pick<
    ReturnType<typeof usePresetChoice>,
    "pending" | "replaceInstructions" | "keepInstructions" | "cancel"
  >;
  selectedPlayer: PlayerData | null | undefined;
  comparePlayer: PlayerData | null | undefined;
  canConfirmSwap: boolean;
  onCloseCompare: () => void;
  onConfirmSwap: () => void;
}

/** The Tactics tab's modals: the preset's question, and the inspector when two players are compared. */
export default function TacticsDialogs({
  presetChoice,
  selectedPlayer,
  comparePlayer,
  canConfirmSwap,
  onCloseCompare,
  onConfirmSwap,
}: TacticsDialogsProps): JSX.Element {
  return (
    <>
      {presetChoice.pending ? (
        <TacticsPresetDialog
          name={presetChoice.pending.name}
          onReplace={() => void presetChoice.replaceInstructions()}
          onKeep={() => void presetChoice.keepInstructions()}
          onCancel={presetChoice.cancel}
        />
      ) : null}

      {/* Inspector modal: only when both players are selected for comparison */}
      {selectedPlayer && comparePlayer ? (
        <TacticsCompareDialog
          canConfirmSwap={canConfirmSwap}
          comparePlayer={comparePlayer}
          onClose={onCloseCompare}
          onConfirmSwap={onConfirmSwap}
          selectedPlayer={selectedPlayer}
        />
      ) : null}
    </>
  );
}
