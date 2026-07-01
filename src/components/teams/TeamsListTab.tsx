import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Search, Trophy, Users } from "lucide-react";

import { formatVal } from "../../lib/helpers";
import { buildRegionLabel } from "../../lib/teamRegions";
import { competitionDisplayName } from "../../lib/competitionName";
import { GameStateData } from "../../store/gameStore";
import {
  fetchTeamsDirectory,
  UNGROUPED_LEAGUE_ID,
  type TeamCard,
  type TeamsDirectory,
} from "../../services/teamsService";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { Badge, Card, CardBody, TeamLocation, TeamLogo } from "../ui";

interface TeamsListTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

interface DisplayLeague {
  id: string;
  rawId: string;
  name: string;
  teams: TeamCard[];
}

interface DisplayRegion {
  id: string;
  name: string;
  leagues: DisplayLeague[];
  teamCount: number;
}

export default function TeamsListTab({ gameState, onSelectTeam }: TeamsListTabProps) {
  const { t, i18n } = useTranslation();
  const userTeamId = gameState.manager.team_id;

  const [search, setSearch] = useState("");
  const [directory, setDirectory] = useState<TeamsDirectory | null>(null);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [expandedRegions, setExpandedRegions] = useState<Set<string>>(new Set());
  const [expandedLeagues, setExpandedLeagues] = useState<Set<string>>(new Set());

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
        setFetchError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
      });
    return () => {
      cancelled = true;
    };
  }, [search, t]);

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
    setExpandedLeagues(
      new Set([`${userLocation.regionId}:${userLocation.leagueId}`]),
    );
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
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
        <input
          type="text"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          placeholder={t("teams.searchPlaceholder")}
          className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 py-2.5 pl-10 pr-4 text-sm text-gray-800 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-primary-500/40"
        />
      </div>

      {fetchError ? (
        <p role="alert" className="text-sm text-red-500">
          {fetchError}
        </p>
      ) : null}

      {visibleGroups.length === 0 ? (
        <p className="py-10 text-center text-sm text-gray-500 dark:text-gray-400">
          {t("teams.noResults")}
        </p>
      ) : (
        visibleGroups.map((region) => {
          const regionOpen = isSearching || expandedRegions.has(region.id);
          return (
            <div key={region.id} className="flex flex-col gap-2">
              <button
                type="button"
                onClick={() => setExpandedRegions((set) => toggle(set, region.id))}
                className="flex items-center gap-2 rounded-lg bg-gray-100 dark:bg-navy-800 px-3 py-2 text-left"
              >
                {regionOpen ? (
                  <ChevronDown className="w-4 h-4 text-gray-500" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-gray-500" />
                )}
                <span className="flex-1 font-heading font-bold uppercase tracking-wide text-sm text-gray-800 dark:text-gray-100">
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
                        onClick={() =>
                          setExpandedLeagues((set) => toggle(set, leagueKey))
                        }
                        className="flex items-center gap-2 rounded-lg px-3 py-1.5 text-left hover:bg-gray-50 dark:hover:bg-navy-700/40"
                      >
                        {leagueOpen ? (
                          <ChevronDown className="w-3.5 h-3.5 text-gray-400" />
                        ) : (
                          <ChevronRight className="w-3.5 h-3.5 text-gray-400" />
                        )}
                        <span className="flex-1 font-heading font-bold uppercase tracking-wider text-xs text-gray-500 dark:text-gray-400">
                          {league.name}
                        </span>
                        <Badge variant="neutral" size="sm">
                          {league.teams.length}
                        </Badge>
                      </button>

                      {leagueOpen && (
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pl-2">
                          {league.teams.map((card) => (
                            <TeamCardView
                              key={card.team.id}
                              card={card}
                              isUser={card.team.id === userTeamId}
                              language={i18n.language}
                              t={t}
                              onSelect={onSelectTeam}
                            />
                          ))}
                        </div>
                      )}
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

function TeamCardView({
  card,
  isUser,
  language,
  t,
  onSelect,
}: {
  card: TeamCard;
  isUser: boolean;
  language: string;
  t: ReturnType<typeof useTranslation>["t"];
  onSelect: (id: string) => void;
}) {
  const {
    team,
    roster_size: rosterSize,
    avg_ovr: avgOvr,
    total_value: totalValue,
    league_pos: leaguePos,
    standing,
  } = card;
  const playStyleLabel = t(`common.playStyles.${team.play_style}`, team.play_style);

  return (
    <Card
      className={`cursor-pointer hover:shadow-lg transition-all ${isUser ? "ring-2 ring-primary-500/30" : ""}`}
    >
      <div onClick={() => onSelect(team.id)} className="overflow-hidden rounded-xl">
        <div
          className="p-5 flex items-center gap-4"
          style={{ background: `linear-gradient(135deg, ${team.colors.primary}, ${team.colors.secondary}40)` }}
        >
          <TeamLogo
            team={team}
            className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-xl text-white border-2 border-white/30 bg-white/15 overflow-hidden"
            imageClassName="h-12 w-12 object-contain drop-shadow"
            style={{ backgroundColor: team.colors.primary }}
          />
          <div className="flex-1 min-w-0">
            <h3 className="font-heading font-bold text-lg text-white uppercase tracking-wide truncate drop-shadow">
              {team.name}
              {isUser && (
                <Badge variant="accent" size="sm" className="ml-2 align-middle">
                  {t("teams.yourTeam")}
                </Badge>
              )}
            </h3>
            <TeamLocation
              city={team.city}
              countryCode={team.country}
              locale={language}
              className="mt-0.5 text-white/70 text-xs"
              iconClassName="w-3 h-3"
              flagClassName="text-xs leading-none"
            />
          </div>
          {leaguePos > 0 && (
            <div className="bg-black/20 backdrop-blur rounded-lg px-3 py-1.5 text-center">
              <p className="text-xs text-white/60 font-heading uppercase tracking-wider">
                {t("common.position")}
              </p>
              <p className="font-heading font-bold text-xl text-white">#{leaguePos}</p>
            </div>
          )}
        </div>

        <div className="grid grid-cols-5 gap-px bg-gray-200 dark:bg-navy-600">
          <StatCell label={t("teams.squad")} value={String(rosterSize)} />
          <StatCell label={t("teams.avgOvr")} value={String(avgOvr)} />
          <StatCell label={t("teams.rep")} value={String(team.reputation)} />
          <StatCell label={t("common.value")} value={formatVal(totalValue)} />
          <StatCell label={t("common.pts")} value={standing ? String(standing.points) : "—"} />
        </div>

        <CardBody>
          <div className="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
            <span className="flex items-center gap-1">
              <Users className="w-3.5 h-3.5" />
              {team.formation} — {playStyleLabel}
            </span>
            <span className="flex items-center gap-1">
              <Trophy className="w-3.5 h-3.5" />
              {t("teams.est")} {team.founded_year}
            </span>
            {standing && (
              <span className="tabular-nums">
                {standing.won}W {standing.drawn}D {standing.lost}L
              </span>
            )}
          </div>
        </CardBody>
      </div>
    </Card>
  );
}

function StatCell({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-white dark:bg-navy-800 px-2 py-2.5 text-center">
      <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
        {label}
      </p>
      <p className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100 mt-0.5">
        {value}
      </p>
    </div>
  );
}
