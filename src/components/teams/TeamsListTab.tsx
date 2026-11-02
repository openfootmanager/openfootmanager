import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Search } from "lucide-react";

import { useListView } from "../../hooks/useListView";
import { nextSort, type SortState } from "../../lib/sortRows";
import { buildRegionLabel } from "../../lib/teamRegions";
import { competitionDisplayName } from "../../lib/competitionName";
import type { GameStateData } from "../../store/gameStore";
import type { ListView } from "../../store/settingsStore";
import {
  fetchTeamsDirectory,
  UNGROUPED_LEAGUE_ID,
  type TeamCard as TeamCardData,
  type TeamsDirectory,
} from "../../services/teamsService";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { Badge, CardGrid, ViewSwitch } from "../ui";
import TeamCard from "./TeamCard";
import TeamsLeagueTable, {
  TEAMS_DEFAULT_SORT,
  type TeamsSortKey,
  teamsSortFirstDirection,
} from "./TeamsLeagueTable";

interface TeamsListTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

interface DisplayLeague {
  id: string;
  rawId: string;
  name: string;
  teams: TeamCardData[];
}

interface DisplayRegion {
  id: string;
  name: string;
  leagues: DisplayLeague[];
  teamCount: number;
}

export default function TeamsListTab({ gameState, onSelectTeam }: TeamsListTabProps) {
  const { t } = useTranslation();
  const userTeamId = gameState.manager.team_id;

  const [search, setSearch] = useState("");
  const [directory, setDirectory] = useState<TeamsDirectory | null>(null);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [expandedRegions, setExpandedRegions] = useState<Set<string>>(new Set());
  const [expandedLeagues, setExpandedLeagues] = useState<Set<string>>(new Set());
  const [view, setView] = useListView();
  // One sort for every league's table, kept for the session as the leagues open and close.
  const [sort, setSort] = useState<SortState<TeamsSortKey>>(TEAMS_DEFAULT_SORT);
  const sortBy = (key: TeamsSortKey) =>
    setSort((current) => nextSort(current, key, teamsSortFirstDirection(key)));

  useEffect(() => {
    let cancelled = false;
    fetchTeamsDirectory({ search: search.trim() || null })
      .then((result) => {
        if (cancelled) return;
        setDirectory(result);
        setFetchError(null);
      })
      .catch((error) => {
        if (cancelled) return;
        setFetchError(getErrorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }, [search]);

  const visibleGroups = useMemo<DisplayRegion[]>(() => {
    if (!directory) return [];
    return directory.regions
      .map((region) => ({
        id: region.id,
        name: buildRegionLabel(t, region.id),
        teamCount: region.team_count,
        leagues: region.leagues.map((league) => ({
          id: league.id,
          rawId: league.id,
          name:
            league.id === UNGROUPED_LEAGUE_ID
              ? t("teams.otherClubs")
              : competitionDisplayName(league, t),
          teams: league.teams,
        })),
      }))
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [directory, t]);

  const userLocation = useMemo(() => {
    if (!directory) return null;
    for (const region of directory.regions) {
      for (const league of region.leagues) {
        if (league.teams.some((card) => card.team.id === userTeamId)) {
          return { regionId: region.id, leagueId: league.id };
        }
      }
    }
    return null;
  }, [directory, userTeamId]);

  const expansionInitialized = useRef(false);
  useEffect(() => {
    if (expansionInitialized.current || !userLocation) return;
    setExpandedRegions(new Set([userLocation.regionId]));
    setExpandedLeagues(new Set([`${userLocation.regionId}:${userLocation.leagueId}`]));
    expansionInitialized.current = true;
  }, [userLocation]);

  const isSearching = search.trim().length > 0;

  const toggle = (set: Set<string>, key: string): Set<string> => {
    const next = new Set(set);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    return next;
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-3">
        <div className="relative min-w-0 flex-1">
          <Search
            aria-hidden="true"
            className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-subtle"
          />
          <input
            type="text"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t("teams.searchPlaceholder")}
            aria-label={t("teams.searchPlaceholder")}
            className="w-full rounded-lg border border-line bg-sunken py-2 pl-9 pr-3 text-body text-fg outline-none placeholder:text-fg-subtle focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
          />
        </div>
        <ViewSwitch value={view} onChange={setView} />
      </div>

      {fetchError ? (
        <p
          role="alert"
          className="rounded-lg border border-line bg-danger-tint px-4 py-2 text-body font-medium text-danger"
        >
          {resolveTranslatedErrorMessage(fetchError, t)}
        </p>
      ) : null}

      {visibleGroups.length === 0 ? (
        <p className="py-10 text-center text-body text-fg-muted">{t("teams.noResults")}</p>
      ) : (
        visibleGroups.map((region) => {
          const regionOpen = isSearching || expandedRegions.has(region.id);
          return (
            <div key={region.id} className="flex flex-col gap-2">
              <button
                type="button"
                onClick={() => setExpandedRegions((set) => toggle(set, region.id))}
                aria-expanded={regionOpen}
                className="flex items-center gap-2 rounded-lg bg-sunken px-3 py-2 text-left hover:bg-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
              >
                {regionOpen ? (
                  <ChevronDown aria-hidden="true" className="h-4 w-4 text-fg-muted" />
                ) : (
                  <ChevronRight aria-hidden="true" className="h-4 w-4 text-fg-muted" />
                )}
                <span className="flex-1 font-heading text-body font-bold uppercase tracking-label text-fg">
                  {region.name}
                </span>
                <Badge variant="neutral" size="sm">
                  {region.teamCount}
                </Badge>
              </button>

              {regionOpen &&
                region.leagues.map((league) => {
                  const leagueKey = `${region.id}:${league.rawId}`;
                  const leagueOpen = isSearching || expandedLeagues.has(leagueKey);
                  return (
                    <div key={leagueKey} className="flex flex-col gap-2 pl-2">
                      <button
                        type="button"
                        onClick={() => setExpandedLeagues((set) => toggle(set, leagueKey))}
                        aria-expanded={leagueOpen}
                        className="flex items-center gap-2 rounded-lg px-3 py-1.5 text-left hover:bg-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
                      >
                        {leagueOpen ? (
                          <ChevronDown aria-hidden="true" className="h-3.5 w-3.5 text-fg-subtle" />
                        ) : (
                          <ChevronRight aria-hidden="true" className="h-3.5 w-3.5 text-fg-subtle" />
                        )}
                        <span className="flex-1 font-heading text-caption font-bold uppercase tracking-label text-fg-muted">
                          {league.name}
                        </span>
                        <Badge variant="neutral" size="sm">
                          {league.teams.length}
                        </Badge>
                      </button>

                      {leagueOpen ? (
                        <LeagueClubs
                          league={league}
                          view={view}
                          userTeamId={userTeamId}
                          sort={sort}
                          onSort={sortBy}
                          onSelectTeam={onSelectTeam}
                        />
                      ) : null}
                    </div>
                  );
                })}
            </div>
          );
        })
      )}
    </div>
  );
}

/** An open league's clubs, as the list view has it: one table, or a grid of cards. */
function LeagueClubs({
  league,
  view,
  userTeamId,
  sort,
  onSort,
  onSelectTeam,
}: {
  league: DisplayLeague;
  view: ListView;
  userTeamId: string | null;
  sort: SortState<TeamsSortKey>;
  onSort: (key: TeamsSortKey) => void;
  onSelectTeam: (id: string) => void;
}) {
  if (view === "table") {
    return (
      <div className="ml-2 overflow-hidden rounded-lg border border-line bg-panel">
        <TeamsLeagueTable
          label={league.name}
          cards={league.teams}
          userTeamId={userTeamId}
          sort={sort}
          onSort={onSort}
          onSelectTeam={onSelectTeam}
        />
      </div>
    );
  }
  return (
    <div className="pl-2">
      <CardGrid>
        {league.teams.map((card) => (
          <li key={card.team.id} className="min-w-0">
            <TeamCard
              card={card}
              isUser={card.team.id === userTeamId}
              onOpen={() => onSelectTeam(card.team.id)}
            />
          </li>
        ))}
      </CardGrid>
    </div>
  );
}
