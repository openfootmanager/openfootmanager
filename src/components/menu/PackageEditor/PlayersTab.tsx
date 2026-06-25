import { useTranslation } from "react-i18next";
import { EntityListShell, EntityRow } from "./shared";
import type { PlayerDef } from "./types";

interface PlayersTabProps {
  players: PlayerDef[];
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

export function PlayersTab({ players, onAdd, onEdit, onDelete, selectedIndex, onSelect }: PlayersTabProps) {
  const { t } = useTranslation();
  return (
    <EntityListShell
      addLabel={t("packageEditor.addPlayer")}
      onAdd={onAdd}
      emptyLabel={t("packageEditor.noPlayers")}
      isEmpty={players.length === 0}
    >
      {players.map((player, i) => (
        <EntityRow
          key={i}
          title={player.name || `${player.firstName} ${player.lastName}`.trim() || player.id}
          subtitle={[t(`common.positions.${player.position}`), player.club]
            .filter(Boolean)
            .join(" · ")}
          badge={
            <div className="w-8 h-8 rounded-lg flex-shrink-0 border border-gray-200 dark:border-navy-600 bg-gray-100 dark:bg-navy-600 flex items-center justify-center">
              <span className="text-[9px] font-heading font-bold text-gray-500 dark:text-gray-400">
                {t(`common.posAbbr.${player.position}`, { defaultValue: player.position.slice(0, 2).toUpperCase() })}
              </span>
            </div>
          }
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("packageEditor.editPlayer")}
          deleteLabel={t("packageEditor.deletePlayer")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
