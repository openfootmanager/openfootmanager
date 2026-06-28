import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { GeneratedAvatar } from "../../ui/GeneratedAvatar";
import { useAssetDataUrl } from "../../../hooks/useAssetDataUrl";
import { POSITION_COLOR } from "./helpers";
import { EntityListShell, EntityRow } from "./shared";
import type { PlayerDef, Position, TeamDef } from "./types";

interface PlayerAvatarCellProps {
  player: PlayerDef;
  posAbbr: string;
  projectDir?: string;
}

function PlayerAvatarCell({ player, posAbbr, projectDir }: PlayerAvatarCellProps) {
  const photoUrl = useAssetDataUrl(player.photo, projectDir);

  const name = player.name || `${player.firstName} ${player.lastName}`.trim() || player.id;
  const posColor = POSITION_COLOR[player.position] ?? "bg-gray-500";

  return (
    <div className="relative flex-shrink-0">
      {photoUrl ? (
        <img
          src={photoUrl}
          alt=""
          className="w-9 h-9 rounded-full object-cover border border-gray-200 dark:border-navy-600"
        />
      ) : (
        <GeneratedAvatar
          name={name}
          initials={name.slice(0, 2).toUpperCase()}
          className="w-9 h-9"
        />
      )}
      <span className={`absolute -bottom-0.5 -right-0.5 text-[7px] font-bold text-white px-0.5 rounded leading-tight ${posColor}`}>
        {posAbbr}
      </span>
    </div>
  );
}

interface PlayersTabProps {
  players: PlayerDef[];
  teams?: TeamDef[];
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
  projectDir?: string;
  youthOnly?: boolean;
}

export function PlayersTab({ players, teams, onAdd, onEdit, onDelete, selectedIndex, onSelect, projectDir, youthOnly }: PlayersTabProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");

  const q = query.trim().toLowerCase();
  const scoped = youthOnly !== undefined
    ? players.map((player, i) => ({ player, i })).filter(({ player }) => !!(player.youth) === youthOnly)
    : players.map((player, i) => ({ player, i }));
  const filtered = q
    ? scoped.filter(({ player }) => {
        const name = (player.name || `${player.firstName} ${player.lastName}`).toLowerCase();
        return (
          name.includes(q) ||
          player.id.toLowerCase().includes(q) ||
          player.club.toLowerCase().includes(q) ||
          player.position.toLowerCase().includes(q) ||
          player.nationality.toLowerCase().includes(q)
        );
      })
    : scoped;

  return (
    <EntityListShell
      addLabel={youthOnly ? t("worldEditor.addYouthPlayer") : t("worldEditor.addPlayer")}
      onAdd={onAdd}
      emptyLabel={youthOnly ? t("worldEditor.noYouthPlayers") : t("worldEditor.noPlayers")}
      isEmpty={scoped.length === 0}
      searchSlot={
        scoped.length > 0 && (
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400 dark:text-gray-500 pointer-events-none" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              aria-label={t("worldEditor.searchPlayers")}
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
          subtitle={[
            t(`common.positions.${player.position}`),
            player.club ? (teams?.find((tm) => tm.id === player.club)?.name ?? player.club) : null,
          ].filter(Boolean).join(" · ")}
          badge={
            <PlayerAvatarCell
              player={player}
              posAbbr={t(`common.posAbbr.${player.position as Position}`, { defaultValue: player.position.slice(0, 2).toUpperCase() })}
              projectDir={projectDir}
            />
          }
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
