import { useTranslation } from "react-i18next";
import type { PlayerData } from "../../store/gameStore";
import { Card } from "../ui";
import { GitCompareArrows } from "lucide-react";
import {
  ComparePlayerAttributes,
  EmptyFocusState,
  PlayerSummaryCard,
  SinglePlayerAttributeGroups,
} from "./TacticsPlayerFocusPanel.sections";

interface TacticsPlayerFocusPanelProps {
  canConfirmSwap: boolean;
  comparePlayer: PlayerData | null;
  currentDate: string;
  onConfirmSwap: () => void;
  selectedPlayer: PlayerData | null;
}

export default function TacticsPlayerFocusPanel({
  canConfirmSwap,
  comparePlayer,
  currentDate,
  onConfirmSwap,
  selectedPlayer,
}: TacticsPlayerFocusPanelProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <div className="p-4 border-b border-gray-100 dark:border-navy-600 bg-linear-to-r from-navy-700 to-navy-800 rounded-t-xl">
        <h3 className="text-sm font-heading font-bold text-white uppercase tracking-wide flex items-center gap-2">
          <GitCompareArrows className="w-4 h-4 text-accent-400" />
          {t("squadCompare.compare", "Compare")}
        </h3>
      </div>
      <div className="p-4">
        {selectedPlayer ? (
          comparePlayer ? (
            <ComparePlayerAttributes
              canConfirmSwap={canConfirmSwap}
              comparePlayer={comparePlayer}
              currentDate={currentDate}
              onConfirmSwap={onConfirmSwap}
              selectedPlayer={selectedPlayer}
            />
          ) : (
            <div className="space-y-4">
              <PlayerSummaryCard
                currentDate={currentDate}
                label={t("tactics.selectedPlayer", "Selected player")}
                player={selectedPlayer}
              />
              <div className="rounded-xl border border-dashed border-gray-200 dark:border-navy-600 px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
                {t(
                  "tactics.selectSecondPlayer",
                  "Select a second player in the lineup view to compare attributes and prepare the swap.",
                )}
              </div>
              <SinglePlayerAttributeGroups player={selectedPlayer} />
            </div>
          )
        ) : (
          <EmptyFocusState />
        )}
      </div>
    </Card>
  );
}
