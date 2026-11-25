import { Search } from "lucide-react";
import { type JSX, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { getTeamName } from "../../lib/helpers";
import type { PlayerData, TeamData } from "../../store/gameStore";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import {
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { Badge } from "../ui";
import { getDashboardSearchResults, getPlayerBadgeVariant } from "./dashboardHelpers";

/** How long a result stays clickable after the field loses focus, so a click on it still lands. */
const SEARCH_CLOSE_DELAY_MS = 200;

function renderSearchResults(props: {
  matchedPlayers: PlayerData[];
  matchedTeams: TeamData[];
  onSelectSearchPlayer: (playerId: string) => void;
  onSelectSearchTeam: (teamId: string) => void;
  teams: TeamData[];
  t: (key: string) => string;
}): JSX.Element {
  const {
    matchedPlayers,
    matchedTeams,
    onSelectSearchPlayer,
    onSelectSearchTeam,
    t,
    teams,
  } = props;

  if (matchedPlayers.length === 0 && matchedTeams.length === 0) {
    return (
      <p className="p-3 text-xs text-gray-400 dark:text-gray-500">
        {t("dashboard.noResults")}
      </p>
    );
  }

  return (
    <>
      {matchedTeams.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
            {t("dashboard.searchTeams")}
          </p>
          {matchedTeams.map((team) => {
            const contextItems = [
              buildViewTeamMenuItem(t, () => onSelectSearchTeam(team.id)),
            ];

            return (
              <ContextMenu items={contextItems} key={team.id}>
                <button
                  onMouseDown={() => onSelectSearchTeam(team.id)}
                  className="flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-gray-50 dark:hover:bg-navy-600"
                  data-testid={`dashboard-search-team-${team.id}`}
                >
                  <div
                    className="flex h-6 w-6 items-center justify-center rounded text-xs font-bold text-white"
                    style={{ backgroundColor: team.colors.primary }}
                  >
                    {team.short_name.charAt(0)}
                  </div>
                  <span className="text-sm font-medium text-gray-800 dark:text-gray-200">
                    {team.name}
                  </span>
                  <span className="ml-auto text-xs text-gray-400">{team.city}</span>
                </button>
              </ContextMenu>
            );
          })}
        </div>
      )}
      {matchedPlayers.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
            {t("dashboard.searchPlayers")}
          </p>
          {matchedPlayers.map((player) => {
            const contextItems: ContextMenuItem[] = [
              buildViewProfileMenuItem(t, () => onSelectSearchPlayer(player.id)),
            ];

            if (player.team_id) {
              contextItems.push(
                buildViewTeamMenuItem(t, () => onSelectSearchTeam(player.team_id!)),
              );
            }

            return (
              <ContextMenu items={contextItems} key={player.id}>
                <button
                  onMouseDown={() => onSelectSearchPlayer(player.id)}
                  className="flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-gray-50 dark:hover:bg-navy-600"
                  data-testid={`dashboard-search-player-${player.id}`}
                >
                  <Badge variant={getPlayerBadgeVariant(player.position)} size="sm">
                    {translatePositionAbbreviation(t, player.position)}
                  </Badge>
                  <span className="text-sm font-medium text-gray-800 dark:text-gray-200">
                    {player.full_name}
                  </span>
                  <span className="ml-auto text-xs text-gray-400">
                    {getTeamName(teams, player.team_id ?? "")}
                  </span>
                </button>
              </ContextMenu>
            );
          })}
        </div>
      )}
    </>
  );
}

interface DashboardSearchProps {
  players: PlayerData[];
  teams: TeamData[];
  onSelectPlayer: (playerId: string) => void;
  onSelectTeam: (teamId: string) => void;
}

/**
 * The header's search: the field and the results under it. It keeps its own query, so a keystroke
 * renders this and nothing above it: the dashboard, and the tab open under it, stay as they were.
 */
export default function DashboardSearch({
  players,
  teams,
  onSelectPlayer,
  onSelectTeam,
}: DashboardSearchProps): JSX.Element {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const results = useMemo(
    () => getDashboardSearchResults({ players, teams }, query),
    [players, teams, query],
  );
  const showResults = open && query.length >= 2;

  useEffect(
    () => () => {
      if (closeTimer.current) clearTimeout(closeTimer.current);
    },
    [],
  );

  function handleFocus(): void {
    if (closeTimer.current) clearTimeout(closeTimer.current);
    setOpen(true);
  }

  function handleBlur(): void {
    closeTimer.current = setTimeout(() => setOpen(false), SEARCH_CLOSE_DELAY_MS);
  }

  function selectPlayer(playerId: string): void {
    onSelectPlayer(playerId);
    setQuery("");
  }

  function selectTeam(teamId: string): void {
    onSelectTeam(teamId);
    setQuery("");
  }

  return (
    <div className="relative mx-auto flex-1 max-w-md">
      <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
      <input
        type="text"
        placeholder={t("dashboard.searchPlaceholder")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onFocus={handleFocus}
        onBlur={handleBlur}
        className="w-full rounded-lg border border-gray-200 bg-gray-100 py-2 pl-9 pr-3 text-sm text-gray-800 transition-all placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500/50 dark:border-navy-600 dark:bg-navy-700 dark:text-gray-200 dark:placeholder-gray-500"
      />
      {showResults && (
        <div className="absolute left-0 right-0 top-full z-30 mt-1 max-h-80 overflow-y-auto rounded-lg border border-gray-200 bg-white shadow-xl dark:border-navy-600 dark:bg-navy-700">
          {renderSearchResults({
            matchedPlayers: results.matchedPlayers,
            matchedTeams: results.matchedTeams,
            onSelectSearchPlayer: selectPlayer,
            onSelectSearchTeam: selectTeam,
            t,
            teams,
          })}
        </div>
      )}
    </div>
  );
}
