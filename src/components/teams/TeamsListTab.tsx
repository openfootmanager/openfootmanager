import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Search, Trophy, Users } from "lucide-react";

import { formatVal } from "../../lib/helpers";
import { GameStateData } from "../../store/gameStore";
import { Badge, Card, CardBody, TeamLocation, TeamLogo } from "../ui";
import {
  buildTeamRegionGroups,
  filterTeamRegionGroups,
  type TeamCard,
} from "./teamsListModel";

interface TeamsListTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

export default function TeamsListTab({ gameState, onSelectTeam }: TeamsListTabProps) {
  const { t, i18n } = useTranslation();
  const userTeamId = gameState.manager.team_id;

  const groups = useMemo(
    () => buildTeamRegionGroups(gameState, t),
    [gameState, t],
  );

  // The user's region/league are expanded by default; everything else stays
  // collapsed so the world's hundreds of clubs render lazily.
  const userLocation = useMemo(() => {
    for (const region of groups) {
      for (const league of region.leagues) {
        if (league.teams.some((card) => card.team.id === userTeamId)) {
          return { regionId: region.id, leagueId: league.id };
        }
      }
    }
    return null;
  }, [groups, userTeamId]);

  const [search, setSearch] = useState("");
  const [expandedRegions, setExpandedRegions] = useState<Set<string>>(
    () => new Set(userLocation ? [userLocation.regionId] : []),
  );
  const [expandedLeagues, setExpandedLeagues] = useState<Set<string>>(
    () => new Set(userLocation ? [`${userLocation.regionId}:${userLocation.leagueId}`] : []),
  );

  const isSearching = search.trim().length > 0;
  const visibleGroups = useMemo(
    () => filterTeamRegionGroups(groups, search),
    [groups, search],
  );

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
    <div className="max-w-6xl mx-auto flex flex-col gap-4">
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
                  const leagueKey = `${region.id}:${league.id}`;
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
  const { team, rosterSize, avgOvr, totalValue, leaguePos, standing } = card;
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
