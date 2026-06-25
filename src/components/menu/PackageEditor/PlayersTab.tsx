import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { GeneratedAvatar } from "../../ui/GeneratedAvatar";
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
  const [query, setQuery] = useState("");

  const q = query.trim().toLowerCase();
  const filtered = q
    ? players.map((player, i) => ({ player, i })).filter(({ player }) => {
        const name = (player.name || `${player.firstName} ${player.lastName}`).toLowerCase();
        return (
          name.includes(q) ||
          player.id.toLowerCase().includes(q) ||
          player.club.toLowerCase().includes(q) ||
          player.position.toLowerCase().includes(q) ||
          player.nationality.toLowerCase().includes(q)
        );
      })
    : players.map((player, i) => ({ player, i }));

  return (
    <EntityListShell
      addLabel={t("worldEditor.addPlayer")}
      onAdd={onAdd}
      emptyLabel={t("worldEditor.noPlayers")}
      isEmpty={players.length === 0}
      searchSlot={
        players.length > 0 && (
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400 dark:text-gray-500 pointer-events-none" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("worldEditor.searchPlayers")}
              className="w-full pl-8 pr-3 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
            />
          </div>
        )
      }
    >
      {filtered.map(({ player, i }) => (
        <EntityRow
          key={i}
          title={player.name || `${player.firstName} ${player.lastName}`.trim() || player.id}
          subtitle={[t(`common.positions.${player.position}`), player.club]
            .filter(Boolean)
            .join(" · ")}
          badge={(() => {
            const name = player.name || `${player.firstName} ${player.lastName}`.trim() || player.id;
            return (
              <div className="relative flex-shrink-0">
                <GeneratedAvatar
                  name={name}
                  initials={name.slice(0, 2).toUpperCase()}
                  className="w-9 h-9"
                />
                <span className={`absolute -bottom-0.5 -right-0.5 text-[7px] font-bold text-white px-0.5 rounded leading-tight ${
                  player.position === "Goalkeeper" ? "bg-amber-500" :
                  ["Defender","CenterBack","RightBack","LeftBack","RightWingBack","LeftWingBack"].includes(player.position) ? "bg-blue-600" :
                  ["Midfielder","DefensiveMidfielder","CentralMidfielder","AttackingMidfielder","RightMidfielder","LeftMidfielder"].includes(player.position) ? "bg-green-600" :
                  "bg-red-600"
                }`}>
                  {t(`common.posAbbr.${player.position}`, { defaultValue: player.position.slice(0, 2).toUpperCase() })}
                </span>
              </div>
            );
          })()}
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("worldEditor.editPlayer")}
          deleteLabel={t("worldEditor.deletePlayer")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
