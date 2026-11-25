import { Search } from "lucide-react";
import {
  type JSX,
  type MouseEvent,
  type RefObject,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import { getTeamName } from "../../lib/helpers";
import { naturalPositionOf } from "../../lib/positions";
import type { PlayerData, TeamData } from "../../store/gameStore";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import {
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import PlayerPositionBadge from "../PlayerPositionBadge";
import { PlayerAvatar, TeamLogo } from "../ui";
import { getDashboardSearchResults } from "./dashboardHelpers";
import { FOCUS_RING, MENU_ROW } from "./dashboardStyles";

/** How long a result stays clickable after the field loses focus, so a click on it still lands. */
const SEARCH_CLOSE_DELAY_MS = 200;

/**
 * A mouse press on a result must not take the focus from the field: WebKit doesn't focus a pressed
 * button, so the field would blur, the search would fold and the click would land on nothing.
 */
function keepFocusInSearch(event: MouseEvent): void {
  event.preventDefault();
}

function renderSearchResults(props: {
  matchedPlayers: PlayerData[];
  matchedTeams: TeamData[];
  onSelectSearchPlayer: (playerId: string) => void;
  onSelectSearchTeam: (teamId: string) => void;
  teams: TeamData[];
  t: (key: string) => string;
}): JSX.Element {
  const { matchedPlayers, matchedTeams, onSelectSearchPlayer, onSelectSearchTeam, t, teams } =
    props;

  if (matchedPlayers.length === 0 && matchedTeams.length === 0) {
    return <p className="p-3 text-caption text-fg-subtle">{t("dashboard.noResults")}</p>;
  }

  return (
    <>
      {matchedTeams.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-caption font-semibold text-fg-muted">
            {t("dashboard.searchTeams")}
          </p>
          {matchedTeams.map((team) => {
            const contextItems = [buildViewTeamMenuItem(t, () => onSelectSearchTeam(team.id))];

            return (
              <ContextMenu items={contextItems} key={team.id}>
                <button
                  type="button"
                  onMouseDown={keepFocusInSearch}
                  onClick={() => onSelectSearchTeam(team.id)}
                  className={MENU_ROW}
                  data-testid={`dashboard-search-team-${team.id}`}
                >
                  <TeamLogo
                    team={team}
                    className="flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-md"
                    imageClassName="h-5 w-5 object-contain"
                  />
                  <span className="text-body font-medium text-fg">{team.name}</span>
                  <span className="ml-auto text-caption text-fg-subtle">{team.city}</span>
                </button>
              </ContextMenu>
            );
          })}
        </div>
      )}
      {matchedPlayers.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-caption font-semibold text-fg-muted">
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
                  type="button"
                  onMouseDown={keepFocusInSearch}
                  onClick={() => onSelectSearchPlayer(player.id)}
                  className={MENU_ROW}
                  data-testid={`dashboard-search-player-${player.id}`}
                >
                  <PlayerAvatar
                    player={player}
                    className="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-md bg-sunken font-heading text-caption font-bold text-fg-muted"
                  />
                  <PlayerPositionBadge position={naturalPositionOf(player)} />
                  <span className="text-body font-medium text-fg">{player.full_name}</span>
                  <span className="ml-auto text-caption text-fg-subtle">
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
  /** Whether the narrow header has opened the field (it folds to a button below 48rem). */
  expanded: boolean;
  /** The focus left the field and its results: a narrow header folds it back to a button. */
  onCollapse: () => void;
  inputRef: RefObject<HTMLInputElement | null>;
  onSelectPlayer: (playerId: string) => void;
  onSelectTeam: (teamId: string) => void;
}

/**
 * The header's search: the field and the results under it. It keeps its own query, so a keystroke
 * renders this and nothing above it: the dashboard, and the page open under it, stay as they were.
 */
export default function DashboardSearch({
  players,
  teams,
  expanded,
  onCollapse,
  inputRef,
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
    // The search folds when the focus leaves the field and its results, not when it moves from
    // the field into a result: a keyboard player tabs into the results to pick one.
    <div
      data-search-field
      className={`relative min-w-0 flex-1 ${expanded ? "" : "hidden @3xl/header:block"}`}
      onBlur={(event) => {
        if (event.currentTarget.contains(event.relatedTarget as Node | null)) return;
        onCollapse();
        handleBlur();
      }}
    >
      <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-subtle" />
      <input
        ref={inputRef}
        type="text"
        placeholder={t("dashboard.searchPlaceholder")}
        aria-label={t("common.search")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onFocus={handleFocus}
        className={`w-full rounded-lg border border-line bg-sunken py-2 pl-9 pr-3 text-body text-fg placeholder:text-fg-subtle ${FOCUS_RING}`}
      />
      {showResults && (
        <div className="absolute left-0 right-0 top-full z-30 mt-1 max-h-80 overflow-y-auto rounded-lg border border-line bg-panel shadow-lg">
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
