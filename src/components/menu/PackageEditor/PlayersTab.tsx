import { useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { GeneratedAvatar } from "../../ui/GeneratedAvatar";
import { useAssetDataUrl } from "../../../hooks/useAssetDataUrl";
import { POSITION_COLOR, entityRowKey } from "./helpers";
import { EntityListFooter, EntityListShell, EntityRow, ExportCsvButton } from "./shared";
import { ENTITY_LIST_PAGE_SIZE, buildTeamNameMap, capRows } from "./entityList.helpers";
import { useKeepRevealed } from "./entityList.reveal";
import { filterPlayerRows, positionFilterGroups, type PositionFilter } from "./PlayersTab.helpers";
import { Select } from "../../ui/Select";
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
  onDuplicate?: (index: number) => void;
  onExportCsv?: () => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
  projectDir?: string;
  youthOnly?: boolean;
}

export function PlayersTab({ players, teams, onAdd, onEdit, onDelete, onDuplicate, onExportCsv, selectedIndex, onSelect, projectDir, youthOnly }: PlayersTabProps) {
  const { t } = useTranslation();
  const positionFilterId = useId();
  const positionFilterCaptionId = useId();
  const [query, setQuery] = useState("");

  const [positionFilter, setPositionFilter] = useState<PositionFilter>("All");
  const [visibleCount, setVisibleCount] = useState(ENTITY_LIST_PAGE_SIZE);

  const teamNames = useMemo(() => buildTeamNameMap(teams), [teams]);
  const { scoped, filtered } = useMemo(
    () => filterPlayerRows({ players, youthOnly, positionFilter, query, teamNames }),
    [players, youthOnly, positionFilter, query, teamNames],
  );
  const { visible } = capRows(
    filtered,
    visibleCount,
    ENTITY_LIST_PAGE_SIZE,
    ({ i }) => i === selectedIndex,
  );
  useKeepRevealed(visible.length, visibleCount, setVisibleCount);

  // Reset here rather than in an effect: an effect would let the old, longer
  // list render once before shrinking it, which is the cost being avoided.
  function handleQueryChange(next: string) {
    setQuery(next);
    setVisibleCount(ENTITY_LIST_PAGE_SIZE);
  }

  function handlePositionFilterChange(next: string) {
    setPositionFilter(next as PositionFilter);
    setVisibleCount(ENTITY_LIST_PAGE_SIZE);
  }

  return (
    <EntityListShell
      addLabel={youthOnly ? t("worldEditor.addYouthPlayer") : t("worldEditor.addPlayer")}
      onAdd={onAdd}
      emptyLabel={youthOnly ? t("worldEditor.noYouthPlayers") : t("worldEditor.noPlayers")}
      isEmpty={scoped.length === 0}
      searchSlot={
        (scoped.length > 0 || onExportCsv) && (
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2">
              {scoped.length > 0 && (
                <div className="relative flex-1">
                  <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400 dark:text-gray-500 pointer-events-none" />
                  <input
                    type="text"
                    value={query}
                    onChange={(e) => handleQueryChange(e.target.value)}
                    aria-label={t("worldEditor.searchPlayers")}
                    placeholder={t("worldEditor.searchPlayers")}
                    className="w-full pl-8 pr-3 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
                  />
                </div>
              )}
              {onExportCsv && <ExportCsvButton onClick={onExportCsv} />}
            </div>
            {scoped.length > 0 && (
              <>
                {/*
                  The list column has no room for a visible caption, but the
                  control still needs a name it can be announced with.
                */}
                <span id={positionFilterCaptionId} className="sr-only">
                  {t("worldEditor.filterByPosition")}
                </span>
              <Select
                selectSize="sm"
                fullWidth
                id={positionFilterId}
                value={positionFilter}
                onChange={(e) => handlePositionFilterChange(e.target.value)}
                // Named after the caption *and* itself, the way CountryCombobox
                // does it, so it reads "<field>, <current value>": a bare
                // aria-label would replace the button's contents, which is
                // where the chosen position is.
                aria-labelledby={`${positionFilterCaptionId} ${positionFilterId}`}
              >
                {/*
                  Flat children, never a fragment: Select reads its options out
                  of `children` and does not descend into one.
                */}
                <option value="All">{t("worldEditor.allPositions")}</option>
                {positionFilterGroups().map((group) => (
                  <optgroup key={group.labelKey} label={t(group.labelKey)}>
                    {group.options.map((option) => (
                      <option key={option.value} value={option.value}>
                        {t(option.labelKey)}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </Select>
              </>
            )}
          </div>
        )
      }
      footerSlot={
        <EntityListFooter
          shown={visible.length}
          matches={filtered.length}
          hasRecords={scoped.length > 0}
          onLoadMore={() => setVisibleCount((n) => n + ENTITY_LIST_PAGE_SIZE)}
        />
      }
    >
      {visible.map(({ player, i }) => (
        <EntityRow
          key={entityRowKey(player.id, i)}
          title={player.name || `${player.firstName} ${player.lastName}`.trim() || player.id}
          subtitle={[
            t(`common.positions.${player.position}`),
            player.club ? (teamNames.get(player.club) ?? player.club) : null,
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
          onDuplicate={onDuplicate ? () => onDuplicate(i) : undefined}
          duplicateLabel={t("worldEditor.duplicateEntity")}
          editLabel={t("worldEditor.editPlayer")}
          deleteLabel={t("worldEditor.deletePlayer")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
