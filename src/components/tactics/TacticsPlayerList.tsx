import type { DragEvent, JSX } from "react";
import { useTranslation } from "react-i18next";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import type { DragState, SquadSection } from "../squad/SquadTab.helpers";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { getPlayerOvr } from "../../lib/helpers";
import { Badge, InjuryBadge } from "../ui";
import ContextMenu from "../ContextMenu";
import { buildTacticsPlayerContextMenuItems } from "./TacticsContextMenu.helpers";
import TacticsFilters from "./TacticsFilters";

interface TacticsPlayerListProps {
  bench: PlayerData[];
  comparePlayerId: string | null;
  dragState: DragState | null;
  matchRoles?: TeamMatchRolesData;
  onAssignMatchRole?: (role: keyof TeamMatchRolesData, playerId: string) => void;
  onClearFilters: () => void;
  onDemoteStarter: (playerId: string) => void;
  onDragEnd: () => void;
  onDragStart: (
    event: DragEvent<HTMLElement>,
    playerId: string,
    from: SquadSection,
    slotIndex: number | null,
  ) => void;
  onOpenPlayerProfile: (playerId: string) => void;
  onPlayerSearchChange: (search: string) => void;
  onPositionFilterChange: (position: string) => void;
  onPromoteBench: (playerId: string) => void;
  onTacticalSelect: (playerId: string, section: SquadSection) => void;
  playerSearch: string;
  positionFilter: string;
  selectedPlayerId: string | null;
  starters: PlayerData[];
  xiActivePosition: Map<string, string>;
}

function PlayerRow({
  comparePlayerId,
  deployedPosition,
  isSelected,
  matchRoles,
  onAssignMatchRole,
  onClearSelection,
  onDemoteStarter,
  onDragEnd,
  onDragStart,
  onOpenPlayerProfile,
  onPromoteBench,
  onTacticalSelect,
  player,
  section,
  selectedPlayerId,
}: {
  comparePlayerId: string | null;
  deployedPosition?: string;
  isSelected: boolean;
  matchRoles?: TeamMatchRolesData;
  onAssignMatchRole?: (role: keyof TeamMatchRolesData, playerId: string) => void;
  onClearSelection: () => void;
  onDemoteStarter?: (playerId: string) => void;
  onDragEnd: () => void;
  onDragStart: (
    event: DragEvent<HTMLElement>,
    playerId: string,
    from: SquadSection,
    slotIndex: number | null,
  ) => void;
  onOpenPlayerProfile: (playerId: string) => void;
  onPromoteBench?: (playerId: string) => void;
  onTacticalSelect: (playerId: string, section: SquadSection) => void;
  player: PlayerData;
  section: SquadSection;
  selectedPlayerId: string | null;
}): JSX.Element {
  const { t } = useTranslation();
  const ovr = getPlayerOvr(player);
  const isCompare = comparePlayerId === player.id;
  // Starters show the slot they are deployed in (issue #272); the natural
  // position remains for bench players, who have no deployed slot.
  const position = translatePositionAbbreviation(
    t,
    deployedPosition || player.natural_position || player.position,
  );
  const contextItems = buildTacticsPlayerContextMenuItems({
    isSelected,
    matchRoles,
    onAssignBestFit: undefined,
    onAssignMatchRole,
    onClearSelection,
    onDemoteStarter,
    onOpenProfile: onOpenPlayerProfile,
    onPromoteBench,
    onTacticalSelect,
    player,
    section,
    selectedPlayerId,
    t,
  });

  const rowClassName = `flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left transition-colors ${
    isSelected
      ? "bg-accent-500/15 ring-1 ring-accent-300/40"
      : isCompare
        ? "bg-primary-500/10 ring-1 ring-primary-300/30"
        : "hover:bg-gray-50 dark:hover:bg-navy-700/50"
  }`;

  if (section === "xi") {
    return (
      <ContextMenu items={contextItems}>
        <button
          type="button"
          data-testid={`xi-player-${player.id}`}
          onClick={() => onTacticalSelect(player.id, "xi")}
          className={rowClassName}
        >
          <Badge
            variant="neutral"
            size="sm"
          >
            {position}
          </Badge>
          <span className="w-6 shrink-0 rounded-md bg-gray-100 py-0.5 text-center text-[11px] font-heading font-bold tabular-nums text-gray-600 dark:bg-navy-700 dark:text-gray-300">
            {player.jersey_number ?? "–"}
          </span>
          <span className="min-w-0 flex-1 truncate text-sm font-medium text-gray-900 dark:text-gray-100">
            {player.match_name || player.full_name}
          </span>
          <span
            className={`shrink-0 rounded-full px-1.5 py-0.5 text-xs font-heading font-bold ${
              ovr >= 80
                ? "bg-primary-500 text-white"
                : ovr >= 60
                  ? "bg-accent-500/20 text-accent-600 dark:text-accent-400"
                  : "bg-gray-100 text-gray-500 dark:bg-navy-700 dark:text-gray-400"
            }`}
          >
            {ovr}
          </span>
        </button>
      </ContextMenu>
    );
  }

  return (
    <ContextMenu items={contextItems}>
      <div data-testid={`bench-player-${player.id}`}>
        <button
          type="button"
          data-testid={`pitch-bench-player-${player.id}`}
          draggable={!player.injury}
          onClick={() => onTacticalSelect(player.id, "bench")}
          onDragStart={(event) => {
            if (!player.injury) {
              onDragStart(event, player.id, "bench", null);
            }
          }}
          onDragEnd={onDragEnd}
          className={`${rowClassName} flex-wrap gap-y-1`}
        >
          <Badge variant="neutral" size="sm">
            {position}
          </Badge>
          <span className="w-6 shrink-0 rounded-md bg-gray-100 py-0.5 text-center text-[11px] font-heading font-bold tabular-nums text-gray-600 dark:bg-navy-700 dark:text-gray-300">
            {player.jersey_number ?? "–"}
          </span>
          <span className="min-w-0 flex-1 truncate text-sm font-medium text-gray-900 dark:text-gray-100">
            {player.match_name || player.full_name}
          </span>
          {player.injury ? (
            <InjuryBadge injury={player.injury} />
          ) : (
            <span
              className={`shrink-0 rounded-full px-1.5 py-0.5 text-xs font-heading font-bold ${
                ovr >= 80
                  ? "bg-primary-500 text-white"
                  : "bg-gray-100 text-gray-500 dark:bg-navy-700 dark:text-gray-400"
              }`}
            >
              {ovr}
            </span>
          )}
        </button>
      </div>
    </ContextMenu>
  );
}

export default function TacticsPlayerList({
  bench,
  comparePlayerId,
  dragState,
  matchRoles,
  onAssignMatchRole,
  onClearFilters,
  onDemoteStarter,
  onDragEnd,
  onDragStart,
  onOpenPlayerProfile,
  onPlayerSearchChange,
  onPositionFilterChange,
  onPromoteBench,
  onTacticalSelect,
  playerSearch,
  positionFilter,
  selectedPlayerId,
  starters,
  xiActivePosition,
}: TacticsPlayerListProps): JSX.Element {
  const { t } = useTranslation();
  const draggedPlayerId = dragState?.playerId ?? null;

  function clearSelection(): void {
    if (selectedPlayerId) {
      // Toggle the current selection off by re-selecting it in its own section;
      // a wrong section would fall through to the compare branch and open the
      // inspector on the player against itself instead of clearing.
      const section: SquadSection = starters.some((p) => p.id === selectedPlayerId)
        ? "xi"
        : "bench";
      onTacticalSelect(selectedPlayerId, section);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <TacticsFilters
        onClear={onClearFilters}
        onPlayerSearchChange={onPlayerSearchChange}
        onPositionFilterChange={onPositionFilterChange}
        playerSearch={playerSearch}
        positionFilter={positionFilter}
      />

      <div className="rounded-xl border border-gray-200 bg-white dark:border-navy-600 dark:bg-navy-800">
        <div className="border-b border-gray-100 px-3 py-2 dark:border-navy-700">
          <span className="text-[11px] font-heading font-bold uppercase tracking-[0.22em] text-gray-500 dark:text-gray-400">
            {t("preMatch.startingXI")} · {starters.length}
          </span>
        </div>
        <div className="p-1.5 space-y-0.5">
          {starters.map((player) => (
            <PlayerRow
              key={player.id}
              comparePlayerId={comparePlayerId}
              deployedPosition={xiActivePosition.get(player.id)}
              isSelected={selectedPlayerId === player.id}
              matchRoles={matchRoles}
              onAssignMatchRole={onAssignMatchRole}
              onClearSelection={clearSelection}
              onDemoteStarter={onDemoteStarter}
              onDragEnd={onDragEnd}
              onDragStart={onDragStart}
              onOpenPlayerProfile={onOpenPlayerProfile}
              onPromoteBench={undefined}
              onTacticalSelect={onTacticalSelect}
              player={player}
              section="xi"
              selectedPlayerId={selectedPlayerId}
            />
          ))}
        </div>
      </div>

      <div className="rounded-xl border border-gray-200 bg-white dark:border-navy-600 dark:bg-navy-800">
        <div className="border-b border-gray-100 px-3 py-2 dark:border-navy-700">
          <span className="text-[11px] font-heading font-bold uppercase tracking-[0.22em] text-gray-500 dark:text-gray-400">
            {t("preMatch.substitutes")} · {bench.length}
          </span>
        </div>
        <div className="p-1.5 space-y-0.5">
          {bench.length === 0 ? (
            <p className="px-2 py-3 text-xs text-gray-500 dark:text-gray-400">
              {t("preMatch.noBench")}
            </p>
          ) : (
            bench.map((player) => (
              <PlayerRow
                key={player.id}
                comparePlayerId={comparePlayerId}
                isSelected={selectedPlayerId === player.id}
                matchRoles={matchRoles}
                onAssignMatchRole={onAssignMatchRole}
                onClearSelection={clearSelection}
                onDemoteStarter={undefined}
                onDragEnd={onDragEnd}
                onDragStart={onDragStart}
                onOpenPlayerProfile={onOpenPlayerProfile}
                onPromoteBench={onPromoteBench}
                onTacticalSelect={onTacticalSelect}
                player={player}
                section="bench"
                selectedPlayerId={selectedPlayerId}
              />
            ))
          )}
        </div>
      </div>

      {draggedPlayerId ? (
        <p className="text-center text-[10px] text-gray-400 dark:text-gray-500">
          {t("squad.dropPlayerHere")}
        </p>
      ) : null}
    </div>
  );
}
