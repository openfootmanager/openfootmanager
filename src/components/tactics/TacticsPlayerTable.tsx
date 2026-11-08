import { MoreVertical } from "lucide-react";
import type { JSX } from "react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useRatingTiers } from "../../hooks/useRatingTiers";
import { contextMenuPoint } from "../../lib/contextMenuPoint";
import { getPlayerOvr } from "../../lib/helpers";
import { getRoleOptions } from "../../lib/playerRoles";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import type { PlayerRole } from "../../store/types";
import type { Familiarity } from "../../store/generated/Familiarity";
import type { TacticsLineup } from "../../store/generated/TacticsLineup";
import ContextMenu, { type ContextMenuHandle } from "../ContextMenu";
import {
  getPreferredPositions,
  type SquadSection,
  translatePositionAbbreviation,
  translatePositionLabel,
} from "../squad/SquadTab.helpers";
import {
  ConditionMeter,
  DataTable,
  type DataTableColumn,
  InjuryBadge,
  PlayerAvatar,
  PositionBadges,
  ProgressBar,
  RatingChip,
  Select,
} from "../ui";
import { buildTacticsPlayerContextMenuItems } from "./TacticsContextMenu.helpers";

/** The table's groups, in the order they are drawn. */
type TableGroup = "xi" | "substitutes" | "reserves";

interface TableRow {
  player: PlayerData;
  group: TableGroup;
  /** The pitch slot a starter plays in; null on the bench. */
  slotIndex: number | null;
  /** The slot's position (a starter's), from the backend's lineup or the formation's layout. */
  position: string | null;
  /** How the starter stands at his slot, the backend's word; null when it hasn't said. */
  familiarity: Familiarity | null;
  /** The role he plays there, the backend's (the one the match will play). */
  role: string | null;
}

/** The screens' badge words for the backend's familiarity. */
const BADGE_FIT = { natural: "natural", can_play: "adapted", out_of_position: "out" } as const;

interface TacticsPlayerTableProps {
  /** The eleven, slot by slot. */
  starters: PlayerData[];
  /** Everyone else in the first-team roster. */
  others: PlayerData[];
  /** Each slot's position, by index, from the formation's layout: the fallback for a slot the backend's lineup doesn't describe yet. */
  slotPositions: readonly string[];
  /** The backend's lineup (`get_tactics_lineup`), or null until it has answered. */
  lineup: TacticsLineup | null;
  playerRoles?: Record<string, string>;
  matchRoles?: TeamMatchRolesData;
  selectedPlayerId: string | null;
  comparePlayerId: string | null;
  onTacticalSelect: (playerId: string, section: SquadSection) => void;
  onRoleChange: (playerId: string, role: PlayerRole) => void;
  onOpenPlayerProfile: (playerId: string) => void;
  onAssignMatchRole?: (role: keyof TeamMatchRolesData, playerId: string) => void;
  onAssignBestFit: (playerId: string) => void;
  onPromoteBench: (playerId: string) => void;
  onDemoteStarter: (playerId: string) => void;
  onClearSelection: () => void;
}

/** The rows in table order: the eleven by slot, the substitutes as the match ranks them, then the reserves. */
export function buildTableRows(
  starters: PlayerData[],
  others: PlayerData[],
  slotPositions: readonly string[],
  lineup: TacticsLineup | null,
  playerRoles: Record<string, string> | undefined,
): TableRow[] {
  const xi = starters.map((player, index): TableRow => {
    // The backend's word for the slot, only while its lineup still names this player there.
    const slot = lineup?.eleven[index]?.player_id === player.id ? lineup.eleven[index] : null;
    return {
      player,
      group: "xi",
      slotIndex: index,
      position: slot?.position ?? slotPositions[index] ?? null,
      familiarity: slot?.familiarity ?? null,
      role: slot?.role ?? playerRoles?.[player.id] ?? null,
    };
  });
  const reserves = new Set(lineup?.reserves ?? []);
  const benchOrder = new Map((lineup?.substitutes ?? []).map((id, index) => [id, index]));
  const rank = (player: PlayerData) => benchOrder.get(player.id) ?? Number.MAX_SAFE_INTEGER;
  const bench = others.map(
    (player): TableRow => ({
      player,
      group: reserves.has(player.id) ? "reserves" : "substitutes",
      slotIndex: null,
      position: null,
      familiarity: null,
      role: null,
    }),
  );
  const substitutes = bench
    .filter((row) => row.group === "substitutes")
    .sort((left, right) => rank(left.player) - rank(right.player));
  return [...xi, ...substitutes, ...bench.filter((row) => row.group === "reserves")];
}

/**
 * The Tactics player table: one row per pitch slot for the eleven, then the substitutes and the
 * reserves. Each row shows its number, position, picture, name, the positions he can play (with
 * the slot's fit), his role there, morale, condition and rating. Picking a name selects him for a
 * swap, as the list did; the row's menu (right-click or its button) holds the rest.
 */
export default function TacticsPlayerTable(props: TacticsPlayerTableProps): JSX.Element {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  const menuRefs = useRef<Map<string, ContextMenuHandle>>(new Map());
  const [openMenuPlayerId, setOpenMenuPlayerId] = useState<string | null>(null);
  const rows = buildTableRows(
    props.starters,
    props.others,
    props.slotPositions,
    props.lineup,
    props.playerRoles,
  );
  const groupTitle: Record<TableGroup, string> = {
    xi: t("preMatch.startingXI"),
    substitutes: t("preMatch.substitutes"),
    reserves: t("tactics.reserves"),
  };
  const columns = tableColumns(props, t, tierOf);

  return (
    <DataTable
      label={t("tactics.playerTable")}
      columns={columns}
      rows={rows}
      rowKey={(row) => row.player.id}
      rowName={(row) => row.player.match_name}
      scrollsItself
      groupOf={(row) => ({ key: row.group, title: groupTitle[row.group] })}
      rowTestId={(row) =>
        row.group === "xi" ? `xi-row-${row.player.id}` : `bench-player-${row.player.id}`
      }
      rowAccent={(row) =>
        row.player.id === props.selectedPlayerId || row.player.id === props.comparePlayerId
          ? "selected"
          : null
      }
      onRowContextMenu={(row, event) => {
        const point = contextMenuPoint(event, event.currentTarget);
        menuRefs.current.get(row.player.id)?.open(point.x, point.y);
      }}
      primaryAside={(row) =>
        row.player.injury ? <InjuryBadge injury={row.player.injury} /> : null
      }
      rowAction={{
        header: t("common.actions"),
        cell: (row) => (
          <ContextMenu
            items={buildTacticsPlayerContextMenuItems({
              isSelected: row.player.id === props.selectedPlayerId,
              matchRoles: props.matchRoles,
              onAssignBestFit: props.onAssignBestFit,
              onAssignMatchRole: props.onAssignMatchRole,
              onClearSelection: props.onClearSelection,
              onDemoteStarter: props.onDemoteStarter,
              onOpenProfile: props.onOpenPlayerProfile,
              onPromoteBench: props.onPromoteBench,
              onTacticalSelect: props.onTacticalSelect,
              player: row.player,
              section: row.group === "xi" ? "xi" : "bench",
              selectedPlayerId: props.selectedPlayerId,
              t,
            })}
            ref={(handle) => {
              if (handle) menuRefs.current.set(row.player.id, handle);
              else menuRefs.current.delete(row.player.id);
            }}
            onOpenChange={(open) =>
              setOpenMenuPlayerId((previous) =>
                open ? row.player.id : previous === row.player.id ? null : previous,
              )
            }
          >
            <button
              type="button"
              onClick={(event) => {
                const rect = event.currentTarget.getBoundingClientRect();
                menuRefs.current.get(row.player.id)?.open(rect.left, rect.bottom + 4);
              }}
              aria-label={t("common.playerActions", { name: row.player.match_name })}
              aria-haspopup="menu"
              aria-expanded={openMenuPlayerId === row.player.id}
              className="rounded-md p-1.5 text-fg-subtle hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
            >
              <MoreVertical aria-hidden="true" className="h-4 w-4" />
            </button>
          </ContextMenu>
        ),
      }}
    />
  );
}

type Translate = ReturnType<typeof useTranslation>["t"];

function tableColumns(
  props: TacticsPlayerTableProps,
  t: Translate,
  tierOf: ReturnType<typeof useRatingTiers>,
): DataTableColumn<TableRow>[] {
  const describe = (position: string) => ({
    label: translatePositionAbbreviation(t, position),
    name: translatePositionLabel(t, position),
  });
  return [
    {
      key: "number",
      header: t("squad.jerseyNumber"),
      headerLabel: t("squad.jerseyNumberFull"),
      numeric: true,
      cell: (row) => row.player.jersey_number ?? "—",
    },
    {
      key: "position",
      header: t("common.position"),
      cell: (row) =>
        row.position ? (
          <span className="font-heading font-bold tracking-label">
            {describe(row.position).label}
          </span>
        ) : (
          <span className="text-fg-subtle">—</span>
        ),
    },
    {
      key: "player",
      header: t("common.player"),
      primary: true,
      cell: (row) => <PlayerName row={row} props={props} />,
    },
    {
      key: "positions",
      header: t("tactics.playablePositions"),
      cell: (row) => {
        const [natural, ...alternates] = getPreferredPositions(row.player);
        return (
          <span className="inline-flex items-center gap-2">
            <PositionBadges
              max={3}
              oneLine
              natural={natural}
              alternates={alternates}
              deployed={row.position ?? undefined}
              familiarity={row.familiarity ? BADGE_FIT[row.familiarity] : undefined}
              describe={describe}
            />
            {row.familiarity === "out_of_position" ? (
              <span className="text-caption font-semibold text-danger">
                {t("squad.outOfPosition")}
              </span>
            ) : null}
          </span>
        );
      },
    },
    {
      key: "role",
      header: t("tactics.playerRoleLabel"),
      cell: (row) => {
        if (row.group !== "xi" || !row.position) return <span className="text-fg-subtle">—</span>;
        const options = getRoleOptions(row.position, row.role);
        return (
          <Select
            selectSize="sm"
            value={row.role ?? "Standard"}
            aria-label={`${t("tactics.playerRoleLabel")}: ${row.player.match_name}`}
            onChange={(event) => {
              const role = options.find((option) => option === event.target.value);
              if (role) props.onRoleChange(row.player.id, role);
            }}
          >
            {options.map((role) => (
              <option key={role} value={role}>
                {t(`tactics.playerRoles.${role}`)}
              </option>
            ))}
          </Select>
        );
      },
    },
    {
      key: "morale",
      header: t("common.morale"),
      numeric: true,
      cell: (row) => (
        <span className="inline-flex w-20 items-center gap-2">
          <span className="w-6 tabular-nums">{row.player.morale}</span>
          <span className="flex-1">
            <ProgressBar value={row.player.morale} variant="muted" size="sm" />
          </span>
        </span>
      ),
    },
    {
      key: "condition",
      header: t("common.condition"),
      cell: (row) => (
        <ConditionMeter value={row.player.condition} status={row.player.condition_status} />
      ),
    },
    {
      key: "ovr",
      header: t("common.ovr"),
      numeric: true,
      cell: (row) => {
        const ovr = getPlayerOvr(row.player);
        return <RatingChip value={ovr} tier={tierOf(ovr)} />;
      },
    },
  ];
}

/** The name: the swap's pick button, with his picture. */
function PlayerName({
  row,
  props,
}: {
  row: TableRow;
  props: TacticsPlayerTableProps;
}): JSX.Element {
  const section: SquadSection = row.group === "xi" ? "xi" : "bench";
  return (
    <button
      type="button"
      data-testid={
        section === "xi" ? `xi-player-${row.player.id}` : `pitch-bench-player-${row.player.id}`
      }
      aria-pressed={row.player.id === props.selectedPlayerId}
      onClick={() => props.onTacticalSelect(row.player.id, section)}
      className="inline-flex min-w-0 items-center gap-2 rounded text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
    >
      <PlayerAvatar player={row.player} />
      <span className="truncate font-semibold">
        {row.player.match_name || row.player.full_name}
      </span>
    </button>
  );
}
