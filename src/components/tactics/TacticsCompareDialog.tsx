import type { PlayerData } from "../../store/gameStore";
import { Dialog } from "../ui";
import TacticsPlayerFocusPanel, { TACTICS_INSPECTOR_TITLE_ID } from "./TacticsPlayerFocusPanel";

interface TacticsCompareDialogProps {
  selectedPlayer: PlayerData;
  comparePlayer: PlayerData;
  canConfirmSwap: boolean;
  onConfirmSwap: () => void;
  onClose: () => void;
}

/**
 * Two players side by side before a swap, over the tactics board: a ui/Dialog, so the focus moves
 * into it, Tab stays inside, and Escape or a click outside it closes it.
 */
export default function TacticsCompareDialog({
  selectedPlayer,
  comparePlayer,
  canConfirmSwap,
  onConfirmSwap,
  onClose,
}: TacticsCompareDialogProps) {
  return (
    <Dialog
      labelledBy={TACTICS_INSPECTOR_TITLE_ID}
      onClose={onClose}
      dismissOnBackdrop
      className="w-full max-w-lg max-h-[85vh] overflow-y-auto rounded-xl"
    >
      <TacticsPlayerFocusPanel
        canConfirmSwap={canConfirmSwap}
        comparePlayer={comparePlayer}
        onClose={onClose}
        onConfirmSwap={onConfirmSwap}
        selectedPlayer={selectedPlayer}
      />
    </Dialog>
  );
}
