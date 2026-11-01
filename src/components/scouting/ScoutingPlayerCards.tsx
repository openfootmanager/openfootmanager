import { ScanSearch } from "lucide-react";
import { useTranslation } from "react-i18next";

import { calcAge, formatVal } from "../../lib/helpers";
import { naturalPositionOf } from "../../lib/positions";
import type { PlayerData, TeamData } from "../../store/gameStore";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import PlayerCard from "../PlayerCard";
import { CardGrid } from "../ui";

interface ScoutActionProps {
  isScouting: boolean;
  availableScoutCount: number;
  sending: boolean;
  onSendScout: () => void;
}

/** What a scouting row or card offers for a player: the scout's progress, no free scout, or Scout. */
export function ScoutAction({
  isScouting,
  availableScoutCount,
  sending,
  onSendScout,
}: ScoutActionProps) {
  const { t } = useTranslation();
  if (isScouting) {
    return (
      <span className="text-caption text-positive font-heading font-bold">
        {t("scouting.scoutingInProgress")}
      </span>
    );
  }
  if (availableScoutCount === 0) {
    return <span className="text-caption text-fg-muted">{t("scouting.noScoutsFree")}</span>;
  }
  return (
    <button
      type="button"
      disabled={sending}
      onClick={onSendScout}
      className="flex items-center gap-1 px-2.5 py-1 rounded-lg bg-primary-500/10 text-primary-500 hover:bg-primary-500/20 transition-colors text-caption font-heading font-bold uppercase tracking-wider disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
    >
      <ScanSearch className="w-3 h-3" />
      {sending ? "..." : t("scouting.scoutBtn")}
    </button>
  );
}

interface ScoutingPlayerCardsProps {
  players: PlayerData[];
  teams: TeamData[];
  alreadyScoutingIds: Set<string>;
  availableScoutCount: number;
  sendingPlayerId: string | null;
  contextItemsFor: (player: PlayerData) => ContextMenuItem[];
  onSelectPlayer?: (id: string) => void;
  onSelectTeam?: (id: string) => void;
  onSendScout: (playerId: string) => void;
}

/**
 * The scouting search as cards. Like its table it shows no rating and no wage (the club has not
 * scouted these players yet), so a card carries the value as its only figure and the scout action.
 */
export default function ScoutingPlayerCards({
  players,
  teams,
  alreadyScoutingIds,
  availableScoutCount,
  sendingPlayerId,
  contextItemsFor,
  onSelectPlayer,
  onSelectTeam,
  onSendScout,
}: ScoutingPlayerCardsProps) {
  const { t } = useTranslation();
  return (
    <CardGrid>
      {players.map((player) => {
        const team = player.team_id ? teams.find((tm) => tm.id === player.team_id) : undefined;
        return (
          <li key={player.id} className="min-w-0">
            <ContextMenu items={contextItemsFor(player)}>
              <div className="contents">
                <PlayerCard
                  portrait={player}
                  name={player.full_name}
                  position={naturalPositionOf(player)}
                  nationality={player.nationality}
                  age={calcAge(player.date_of_birth)}
                  teamName={player.team_id ? (team?.name ?? "") : t("common.freeAgent")}
                  onSelectTeam={
                    player.team_id && onSelectTeam
                      ? () => onSelectTeam(player.team_id as string)
                      : undefined
                  }
                  kit={team?.colors ?? null}
                  status={
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-fg-muted">
                        {t("scouting.value")}{" "}
                        <span className="font-semibold text-fg tabular-nums">
                          {formatVal(player.market_value)}
                        </span>
                      </span>
                      <ScoutAction
                        isScouting={alreadyScoutingIds.has(player.id)}
                        availableScoutCount={availableScoutCount}
                        sending={sendingPlayerId === player.id}
                        onSendScout={() => onSendScout(player.id)}
                      />
                    </div>
                  }
                  onOpen={onSelectPlayer ? () => onSelectPlayer(player.id) : undefined}
                />
              </div>
            </ContextMenu>
          </li>
        );
      })}
    </CardGrid>
  );
}
