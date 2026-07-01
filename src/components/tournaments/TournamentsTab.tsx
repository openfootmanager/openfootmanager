import { useState, useEffect } from "react";
import CompetitionsOverview from "./CompetitionsOverview";
import KnockoutBracket from "./KnockoutBracket";
import { invoke } from "@tauri-apps/api/core";
import type { ReactNode } from "react";
import type { TFunction } from "i18next";
import {
  FixtureData,
  GameStateData,
  LeagueData,
  SeasonAwardEntryData,
  SeasonAwardsData,
  SeasonManagerAwardEntryData,
} from "../../store/gameStore";
import {
  fetchCompetitionsView,
  type CompetitionsView,
} from "../../services/competitionsService";
import ContextMenu from "../ContextMenu";
import { competitionDisplayName } from "../../lib/competitionName";
import { Card, CardHeader, CardBody, Badge, Select } from "../ui";
import {
  Trophy,
  Calendar,
  TableProperties,
  Award,
  Briefcase,
  GitBranch,
  Star,
  Shield,
  Users,
  Zap,
} from "lucide-react";
import {
  getCompetitiveFixtures,
  getPromotionRelegationZones,
  formatMatchDate,
} from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { useTranslation } from "react-i18next";
import {
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";

interface TournamentsTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
  onSelectPlayer?: (id: string) => void;
}

function isKnockoutCompetition(competition: LeagueData): boolean {
  return (
    (competition.rules != null && competition.rules.format !== "LeagueTable") ||
    (competition.knockout_rounds?.length ?? 0) > 0
  );
}

function byTablePosition(
  a: { points: number; goals_for: number; goals_against: number },
  b: { points: number; goals_for: number; goals_against: number },
): number {
  return (
    b.points - a.points ||
    b.goals_for - b.goals_against - (a.goals_for - a.goals_against) ||
    b.goals_for - a.goals_for
  );
}

/** Round names arrive as backend data ("Final", "Round of 16"); localize the known shapes. */
function localizedRoundName(t: TFunction, name: string): string {
  if (name === "Final") return t("tournaments.rounds.final");
  if (name === "Semifinal") return t("tournaments.rounds.semifinal");
  if (name === "Quarterfinal") return t("tournaments.rounds.quarterfinal");
  const roundOf = name.match(/^Round of (\d+)$/);
  if (roundOf) return t("tournaments.rounds.roundOf", { size: roundOf[1] });
  return name;
}

export default function TournamentsTab({
  gameState,
  onSelectTeam,
  onSelectPlayer,
}: TournamentsTabProps) {
  const { t } = useTranslation();
  const [competitionsView, setCompetitionsView] = useState<CompetitionsView | null>(null);

  const currentDate = gameState.clock?.current_date;

  useEffect(() => {
    let cancelled = false;
    fetchCompetitionsView()
      .then((view) => {
        if (!cancelled) setCompetitionsView(view);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [currentDate]);

  // Fall back to building a teamNames map from gameState.teams while slice loads.
  const fallbackTeamNames: Record<string, string> = Object.fromEntries(
    (gameState.teams ?? []).map((t) => [t.id, t.name]),
  );
  const teamNames = competitionsView?.team_names ?? fallbackTeamNames;
  const fallbackNationalTeamNames: Record<string, string> = Object.fromEntries(
    (gameState.national_teams ?? []).map((nt) => [nt.id, nt.name]),
  );
  const nationalTeamNames =
    competitionsView?.national_team_names ?? fallbackNationalTeamNames;
  const nationalTeamNameKeys = competitionsView?.national_team_name_keys ?? {};
  const playerNames = competitionsView?.player_names ?? {};

  const userTeamId =
    competitionsView?.manager_team_id ?? gameState.manager.team_id;
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";

  // Derive active competitions from slice; fall back to gameState while loading.
  const gsLeague = gameState.league ? [gameState.league] : [];
  const allCompetitions =
    competitionsView?.competitions ?? gameState.competitions ?? gsLeague;
  const activeIds =
    competitionsView?.active_competition_ids ?? gameState.active_competition_ids ?? [];
  const activeCompetitions =
    activeIds.length === 0
      ? allCompetitions
      : allCompetitions.filter((c) => activeIds.includes(c.id));

  const [view, setView] = useState<
    "overview" | "fixtures" | "standings" | "awards"
  >("overview");
  const [selectedCompetitionId, setSelectedCompetitionId] = useState<string | null>(null);
  const [awardsBySeason, setAwardsBySeason] = useState<
    Record<number, SeasonAwardsData>
  >({});
  const [awardsLoadState, setAwardsLoadState] = useState<
    "idle" | "loading" | "error"
  >("idle");
  const [awardsRetryCount, setAwardsRetryCount] = useState(0);
  const userCompetitions = activeCompetitions.filter((competition) =>
    competition.participant_ids?.includes(userTeamId ?? ""),
  );
  const league =
    activeCompetitions.find((competition) => competition.id === selectedCompetitionId) ??
    userCompetitions[0] ??
    activeCompetitions[0] ??
    gameState.league ??
    null;
  const currentSeason = league?.season ?? 0;
  const awards = awardsBySeason[currentSeason] ?? null;
  const isWorldCup = league?.kind === "InternationalNation";
  const worldCupChampions = competitionsView?.world_cup_champions ?? gameState.world_history?.world_cup_champions ?? [];
  const worldCupChampion = isWorldCup
    ? worldCupChampions.find((c) => c.year === currentSeason) ?? null
    : null;

  const activeCompetitionIds = activeCompetitions.map((c) => c.id).join(",");
  const userCompetitionIds = userCompetitions.map((c) => c.id).join(",");

  useEffect(() => {
    if (activeCompetitions.length === 0) {
      if (selectedCompetitionId !== null) {
        setSelectedCompetitionId(null);
      }
      return;
    }

    const hasSelection = activeCompetitions.some(
      (competition) => competition.id === selectedCompetitionId,
    );
    if (hasSelection) {
      return;
    }

    setSelectedCompetitionId(userCompetitions[0]?.id ?? activeCompetitions[0].id);
  // activeCompetitionIds / userCompetitionIds are stable string keys derived from
  // the arrays; using the arrays directly would cause the effect to fire on every
  // render because getActiveCompetitions() and .filter() always return new refs.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeCompetitionIds, selectedCompetitionId, userCompetitionIds]);

  useEffect(() => {
    if (view !== "awards" || awards) {
      return;
    }

    let cancelled = false;
    setAwardsLoadState("loading");

    invoke<SeasonAwardsData>("get_season_awards")
      .then((nextAwards) => {
        if (cancelled) {
          return;
        }

        setAwardsBySeason((current) => ({
          ...current,
          [currentSeason]: nextAwards,
        }));
        setAwardsLoadState("idle");
      })
      .catch(() => {
        if (!cancelled) {
          setAwardsLoadState("error");
        }
      });

    return () => {
      cancelled = true;
    };
  }, [view, awards, currentSeason, awardsRetryCount]);

  if (!league) {
    return (
      <div className="max-w-4xl mx-auto text-center py-12">
        <Trophy className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
        <p className="text-gray-500 dark:text-gray-400 text-sm">
          {t("tournaments.noActive")}
        </p>
      </div>
    );
  }

  const standings = [...league.standings].sort(byTablePosition);

  const isKnockout = isKnockoutCompetition(league);
  const knockoutRounds = league.knockout_rounds ?? [];
  const groups = league.groups ?? [];
  const zones = isKnockout
    ? { promotionSlots: 0, relegationSlots: 0 }
    : getPromotionRelegationZones(activeCompetitions, league);
  const participantCount = league.participant_ids?.length ?? league.standings.length;

  const competitiveFixtures = getCompetitiveFixtures(league.fixtures);

  const matchdays = new Map<number, FixtureData[]>();
  competitiveFixtures.forEach((f) => {
    const list = matchdays.get(f.matchday) || [];
    list.push(f);
    matchdays.set(f.matchday, list);
  });
  const sortedMatchdays = Array.from(matchdays.entries()).sort(
    (a, b) => a[0] - b[0],
  );

  const completedMatchdays = sortedMatchdays.filter(([, fixtures]) =>
    fixtures.every((f) => f.status === "Completed"),
  ).length;
  const totalMatchdays = sortedMatchdays.length;
  // Awards only become final once the competition's season has fully played out;
  // before that the standings-based winners are just current leaders.
  const seasonComplete = totalMatchdays > 0 && completedMatchdays >= totalMatchdays;
  const totalGoals = competitiveFixtures
    .filter((f) => f.result)
    .reduce((s, f) => s + (f.result!.home_goals + f.result!.away_goals), 0);
  const completedMatches = competitiveFixtures.filter(
    (f) => f.status === "Completed",
  ).length;

  // Build fallback player name lookup from gameState.players while slice loads.
  const fallbackPlayerNames = Object.fromEntries(
    (gameState.players ?? []).map((p) => [
      p.id,
      {
        match_name: p.match_name,
        full_name: p.full_name,
        team_id: p.team_id ?? null,
        team_name: teamNames[p.team_id ?? ""] ?? null,
      },
    ]),
  );
  const resolvedPlayerNames =
    Object.keys(playerNames).length > 0 ? playerNames : fallbackPlayerNames;

  const topScorers = (() => {
    const goals: Record<string, number> = {};
    competitiveFixtures.forEach((f) => {
      if (f.result) {
        f.result.home_scorers.forEach((s) => {
          goals[s.player_id] = (goals[s.player_id] || 0) + 1;
        });
        f.result.away_scorers.forEach((s) => {
          goals[s.player_id] = (goals[s.player_id] || 0) + 1;
        });
      }
    });
    return Object.entries(goals)
      .map(([pid, g]) => ({
        playerId: pid,
        playerName: resolvedPlayerNames[pid] ?? null,
        goals: g,
      }))
      .filter((e) => e.playerName !== null)
      .sort((a, b) => b.goals - a.goals)
      .slice(0, 10);
  })();

  const isClubTeam = (id: string) => id in teamNames;
  const resolveTeamName = (id: string) => {
    if (id in teamNames) return teamNames[id];
    const nameKey = nationalTeamNameKeys[id];
    if (nameKey) return t("nations.nationalTeamTemplate", { name: t(nameKey) });
    return nationalTeamNames[id] ?? id;
  };

  const buildFixtureMenuItems = (fixture: FixtureData) =>
    [fixture.home_team_id, fixture.away_team_id]
      .filter((teamId) => isClubTeam(teamId))
      .map((teamId) => ({
        ...buildViewTeamMenuItem(t, () => onSelectTeam(teamId)),
        label: `${t("common.viewTeam")}: ${resolveTeamName(teamId)}`,
      }));

  const buildStandingMenuItems = (teamId: string) => [
    buildViewTeamMenuItem(t, () => onSelectTeam(teamId)),
  ];

  const buildPlayerMenuItems = (playerId: string, teamId?: string | null) => {
    const items = [];

    if (typeof onSelectPlayer === "function") {
      items.push(buildViewProfileMenuItem(t, () => onSelectPlayer(playerId)));
    }

    if (teamId) {
      items.push(buildViewTeamMenuItem(t, () => onSelectTeam(teamId)));
    }

    return items;
  };

  const renderGroupTable = (group: NonNullable<LeagueData["groups"]>[number]) => {
    const groupStandings = [...group.standings].sort(byTablePosition);
    return (
      <div key={group.id} data-testid={`tournaments-group-${group.id}`}>
        <div className="px-4 py-2 border-b border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800">
          <h5 className="font-heading font-bold text-xs uppercase tracking-wider text-gray-600 dark:text-gray-300">
            {t("tournaments.group", { name: group.name })}
          </h5>
        </div>
        <table className="w-full text-left border-collapse">
          <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
            {groupStandings.map((entry, idx) => {
              const isUser = entry.team_id === userTeamId;
              return (
                <tr
                  key={entry.team_id}
                  onClick={
                    isClubTeam(entry.team_id)
                      ? () => onSelectTeam(entry.team_id)
                      : undefined
                  }
                  className={`${isClubTeam(entry.team_id) ? "cursor-pointer" : ""} transition-colors ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}
                  data-testid={`tournaments-group-standing-${entry.team_id}`}
                >
                  <td className="py-1.5 px-3 font-heading font-bold text-xs text-gray-400 w-6">
                    {idx + 1}
                  </td>
                  <td
                    className={`py-1.5 px-3 font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                  >
                    {resolveTeamName(entry.team_id)}
                  </td>
                  <td className="py-1.5 px-3 text-center text-xs text-gray-600 dark:text-gray-400 tabular-nums">
                    {entry.played}
                  </td>
                  <td className="py-1.5 px-3 text-center font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums">
                    {entry.points}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    );
  };

  const renderFixtureRow = (f: FixtureData, testId: string) => {
    const isUserMatch =
      f.home_team_id === userTeamId || f.away_team_id === userTeamId;
    const completed = f.status === "Completed";
    return (
      <ContextMenu items={buildFixtureMenuItems(f)} key={f.id}>
        <div
          className={`flex items-center px-5 py-3 transition-colors ${isUserMatch ? "bg-primary-50/50 dark:bg-primary-500/5" : ""}`}
          data-testid={testId}
        >
          <span
            onClick={
              isClubTeam(f.home_team_id)
                ? () => onSelectTeam(f.home_team_id)
                : undefined
            }
            className={`flex-1 text-right font-semibold text-sm ${isClubTeam(f.home_team_id) ? "cursor-pointer hover:underline" : ""} ${f.home_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
          >
            {resolveTeamName(f.home_team_id)}
          </span>
          <div className="w-24 text-center mx-3">
            {completed && f.result ? (
              <span className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100">
                {f.result.home_goals} - {f.result.away_goals}
              </span>
            ) : (
              <Badge variant="neutral" size="sm">
                vs
              </Badge>
            )}
          </div>
          <span
            onClick={
              isClubTeam(f.away_team_id)
                ? () => onSelectTeam(f.away_team_id)
                : undefined
            }
            className={`flex-1 text-left font-semibold text-sm ${isClubTeam(f.away_team_id) ? "cursor-pointer hover:underline" : ""} ${f.away_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
          >
            {resolveTeamName(f.away_team_id)}
          </span>
        </div>
      </ContextMenu>
    );
  };

  return (
    <div>
      {isPreseason && (
        <Card accent="accent" className="mb-5">
          <CardBody>
            <div className="flex flex-col gap-1.5">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="accent" size="sm">
                  {t(`season.phases.${seasonContext.phase}`)}
                </Badge>
                <span className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                  {seasonContext.season_start
                    ? t("season.startsOn", {
                      date: formatMatchDate(seasonContext.season_start),
                    })
                    : t("season.noOpener")}
                </span>
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                {t("season.tournamentsPreseasonHint")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      {/* League header */}
      <Card accent="primary" className="mb-5">
        <div className="bg-gradient-to-r from-navy-700 to-navy-800 p-6 rounded-t-xl">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center">
            <div className="w-14 h-14 rounded-xl bg-accent-500/20 flex items-center justify-center">
              <Trophy className="w-7 h-7 text-accent-400" />
            </div>
            <div className="flex-1">
              <h2 className="text-2xl font-heading font-bold text-white uppercase tracking-wide">
                {competitionDisplayName(league, t)}
              </h2>
              <p className="text-gray-400 text-sm mt-0.5">
                {t("schedule.season", { number: league.season })} —{" "}
                {t("tournaments.nTeams", { count: participantCount })}
              </p>
            </div>
            {activeCompetitions.length > 1 && (
              <Select
                value={league.id}
                onChange={(event) => setSelectedCompetitionId(event.target.value)}
                variant="ghost"
                aria-label={t("common.competition")}
              >
                {activeCompetitions.map((competition) => (
                  <option
                    key={competition.id}
                    value={competition.id}
                  >
                    {competitionDisplayName(competition, t)}
                  </option>
                ))}
              </Select>
            )}
            <div className="hidden md:flex gap-4">
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("tournaments.progress")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {completedMatchdays}/{totalMatchdays}
                </p>
              </div>
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("tournaments.matches")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {completedMatches}
                </p>
              </div>
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("tournaments.goals")}
                </p>
                <p className="font-heading font-bold text-lg text-accent-400">
                  {totalGoals}
                </p>
              </div>
            </div>
          </div>
        </div>
        {worldCupChampion && (
          <div className="flex items-center gap-3 bg-accent-500/10 px-6 py-3 rounded-b-xl border-t border-accent-500/20">
            <Trophy className="w-5 h-5 text-accent-400 flex-shrink-0" />
            <span className="text-sm font-heading font-bold uppercase tracking-wider text-accent-300">
              {t("tournaments.worldCupChampion")}:
            </span>
            <span className="text-sm font-semibold text-white">
              {worldCupChampion.nation_name}
            </span>
          </div>
        )}
      </Card>

      {/* Tab switcher */}
      <div className="flex gap-2 mb-5">
        {(["overview", "standings", "fixtures", "awards"] as const).map((v) => (
          <button
            key={v}
            onClick={() => setView(v)}
            className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all ${view === v
              ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
              : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 border border-gray-200 dark:border-navy-600"
              }`}
          >
            {v === "overview" ? (
              <>
                <Trophy className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                {t("tournaments.overview")}
              </>
            ) : v === "standings" ? (
              isKnockout ? (
                <>
                  <GitBranch className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                  {t("tournaments.bracket")}
                </>
              ) : (
                <>
                  <TableProperties className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                  {t("schedule.standings")}
                </>
              )
            ) : v === "awards" ? (
              <>
                <Award className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                {t("tournaments.awardsTab")}
              </>
            ) : (
              <>
                <Calendar className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                {t("schedule.fixtures")}
              </>
            )}
          </button>
        ))}
      </div>

      {/* Overview */}
      {view === "overview" && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
          {/* Mini standings / bracket progress */}
          <Card className="lg:col-span-2">
            <CardHeader>
              {isKnockout ? t("tournaments.bracket") : t("tournaments.leagueTable")}
            </CardHeader>
            <CardBody className="p-0">
              {isKnockout ? (
                knockoutRounds.length === 0 && groups.length > 0 ? (
                  <div className="grid grid-cols-1 md:grid-cols-2">
                    {groups.map(renderGroupTable)}
                  </div>
                ) : (
                  <div className="divide-y divide-gray-100 dark:divide-navy-600">
                    {knockoutRounds.map((round) => (
                      <div
                        key={round.id}
                        className="flex items-center justify-between px-4 py-2.5"
                        data-testid={`tournaments-round-summary-${round.id}`}
                      >
                        <span className="text-sm font-semibold text-gray-800 dark:text-gray-200">
                          {localizedRoundName(t, round.name)}
                        </span>
                        <Badge variant={round.completed ? "accent" : "neutral"} size="sm">
                          {round.completed
                            ? t("tournaments.roundComplete")
                            : t("tournaments.roundInProgress")}
                        </Badge>
                      </div>
                    ))}
                  </div>
                )
              ) : groups.length > 0 ? (
                <div className="grid grid-cols-1 md:grid-cols-2">
                  {groups.map(renderGroupTable)}
                </div>
              ) : isPreseason ? (
                <div className="flex flex-col items-center gap-2 px-6 py-8 text-center">
                  <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
                  <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                    {t("season.standingsLocked")}
                  </p>
                  <p className="text-xs text-gray-500 dark:text-gray-400 max-w-md">
                    {t("season.tournamentsPreseasonHint")}
                  </p>
                </div>
              ) : (
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 w-8">
                        #
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.team")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.played")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.won")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.drawn")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.lost")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.gd")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.pts")}
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                    {standings.map((entry, idx) => {
                      const isUser = entry.team_id === userTeamId;
                      const gd = entry.goals_for - entry.goals_against;
                      return (
                        <ContextMenu
                          items={buildStandingMenuItems(entry.team_id)}
                          key={entry.team_id}
                        >
                          <tr
                            onClick={() => onSelectTeam(entry.team_id)}
                            className={`cursor-pointer transition-colors ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}
                            data-testid={`tournaments-overview-standing-${entry.team_id}`}
                          >
                            <td className="py-2 px-3 font-heading font-bold text-sm text-gray-400">
                              {idx + 1}
                            </td>
                            <td
                              className={`py-2 px-3 font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                            >
                              {resolveTeamName(entry.team_id)}
                            </td>
                            <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                              {entry.played}
                            </td>
                            <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                              {entry.won}
                            </td>
                            <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                              {entry.drawn}
                            </td>
                            <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                              {entry.lost}
                            </td>
                            <td
                              className={`py-2 px-3 text-center text-sm font-semibold tabular-nums ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500"}`}
                            >
                              {gd > 0 ? `+${gd}` : gd}
                            </td>
                            <td className="py-2 px-3 text-center font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums">
                              {entry.points}
                            </td>
                          </tr>
                        </ContextMenu>
                      );
                    })}
                  </tbody>
                </table>
              )}
            </CardBody>
          </Card>

          {/* Top scorers */}
          <Card>
            <CardHeader>{t("tournaments.topScorers")}</CardHeader>
            <CardBody className="p-0">
              {topScorers.length === 0 ? (
                <p className="p-4 text-sm text-gray-400 dark:text-gray-500 text-center">
                  {t("tournaments.noGoals")}
                </p>
              ) : (
                <div className="divide-y divide-gray-100 dark:divide-navy-600">
                  {topScorers.map((entry, i) => (
                    <ContextMenu
                      items={buildPlayerMenuItems(
                        entry.playerId,
                        entry.playerName!.team_id,
                      )}
                      key={entry.playerId}
                    >
                      <div
                        className="flex items-center px-4 py-2.5 gap-3"
                        data-testid={`tournaments-top-scorer-${entry.playerId}`}
                      >
                        <span className="font-heading font-bold text-sm text-gray-400 dark:text-gray-500 w-5 text-center">
                          {i + 1}
                        </span>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
                            {entry.playerName!.full_name}
                          </p>
                          <p className="text-xs text-gray-400 dark:text-gray-500">
                            {entry.playerName!.team_name ?? entry.playerName!.team_id ?? ""}
                          </p>
                        </div>
                        <span className="font-heading font-bold text-lg text-accent-500 tabular-nums">
                          {entry.goals}
                        </span>
                      </div>
                    </ContextMenu>
                  ))}
                </div>
              )}
            </CardBody>
          </Card>
        </div>
      )}

      {view === "overview" && activeCompetitions.length > 1 && (
        <div className="mt-5">
          <CompetitionsOverview
            competitions={activeCompetitions}
            userTeamId={userTeamId}
            onSelect={setSelectedCompetitionId}
          />
        </div>
      )}

      {/* Knockout bracket (with group stage when present) */}
      {view === "standings" && isKnockout && (
        <div className="flex flex-col gap-4">
          {groups.length > 0 && (
            <Card>
              <CardBody className="p-0">
                <div className="grid grid-cols-1 md:grid-cols-2">
                  {groups.map(renderGroupTable)}
                </div>
              </CardBody>
            </Card>
          )}
          {knockoutRounds.length > 0 && (
            <KnockoutBracket
              rounds={knockoutRounds}
              fixtures={league.fixtures}
              resolveTeamName={resolveTeamName}
              localizedRoundName={(name) => localizedRoundName(t, name)}
              userTeamId={userTeamId}
              roundCompleteLabel={t("tournaments.roundComplete")}
              roundInProgressLabel={t("tournaments.roundInProgress")}
              byeLabel={t("tournaments.bye")}
              tbdLabel={t("tournaments.tbd")}
            />
          )}
        </div>
      )}

      {/* Group tables for non-knockout competitions (e.g. World Cup qualifying) */}
      {view === "standings" && !isKnockout && groups.length > 0 && (
        <Card>
          <CardBody className="p-0">
            <div className="grid grid-cols-1 md:grid-cols-2">
              {groups.map(renderGroupTable)}
            </div>
          </CardBody>
        </Card>
      )}

      {/* Full standings */}
      {view === "standings" && !isKnockout && groups.length === 0 &&
        (isPreseason ? (
          <Card>
            <CardBody>
              <div className="flex flex-col items-center gap-2 py-6 text-center">
                <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
                <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                  {t("season.standingsLocked")}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400 max-w-md">
                  {t("season.tournamentsPreseasonHint")}
                </p>
              </div>
            </CardBody>
          </Card>
        ) : (
          <Card>
            <div className="p-5 border-b border-gray-100 dark:border-navy-600 bg-gradient-to-r from-navy-700 to-navy-800 rounded-t-xl">
              <h3 className="text-lg font-heading font-bold text-white flex items-center gap-2 uppercase tracking-wide">
                <Trophy className="text-accent-400 w-5 h-5" />
                {competitionDisplayName(league, t)} —{" "}
                {t("schedule.season", { number: league.season })}
              </h3>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse">
                <thead>
                  <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 w-8">
                      #
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {t("common.team")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.played")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.won")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.drawn")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.lost")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.gf")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.ga")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.gd")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.pts")}
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                  {standings.map((entry, idx) => {
                    const isUser = entry.team_id === userTeamId;
                    const gd = entry.goals_for - entry.goals_against;
                    const inPromotionZone = idx < zones.promotionSlots;
                    const inRelegationZone =
                      zones.relegationSlots > 0 &&
                      idx >= standings.length - zones.relegationSlots;
                    return (
                      <ContextMenu
                        items={buildStandingMenuItems(entry.team_id)}
                        key={entry.team_id}
                      >
                        <tr
                          onClick={() => onSelectTeam(entry.team_id)}
                          className={`cursor-pointer transition-colors ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}
                          data-testid={`tournaments-standing-${entry.team_id}`}
                        >
                          <td
                            className={`py-3 px-4 font-heading font-bold text-sm ${
                              inPromotionZone
                                ? "border-l-2 border-primary-500 text-primary-500"
                                : inRelegationZone
                                  ? "border-l-2 border-red-500 text-red-500"
                                  : "text-gray-400"
                            }`}
                            data-testid={
                              inPromotionZone
                                ? `tournaments-promotion-${entry.team_id}`
                                : inRelegationZone
                                  ? `tournaments-relegation-${entry.team_id}`
                                  : undefined
                            }
                          >
                            {idx + 1}
                          </td>
                          <td
                            className={`py-3 px-4 font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                          >
                            {resolveTeamName(entry.team_id)}
                          </td>
                          <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.played}
                          </td>
                          <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.won}
                          </td>
                          <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.drawn}
                          </td>
                          <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.lost}
                          </td>
                          <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.goals_for}
                          </td>
                          <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.goals_against}
                          </td>
                          <td
                            className={`py-3 px-4 text-center text-sm font-semibold tabular-nums ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500"}`}
                          >
                            {gd > 0 ? `+${gd}` : gd}
                          </td>
                          <td className="py-3 px-4 text-center font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums">
                            {entry.points}
                          </td>
                        </tr>
                      </ContextMenu>
                    );
                  })}
                </tbody>
              </table>
              {(zones.promotionSlots > 0 || zones.relegationSlots > 0) && (
                <div className="flex gap-5 border-t border-gray-100 px-4 py-2.5 text-xs text-gray-500 dark:border-navy-600 dark:text-gray-400">
                  {zones.promotionSlots > 0 && (
                    <span className="flex items-center gap-1.5">
                      <span className="h-2 w-2 rounded-full bg-primary-500" />
                      {t("schedule.promotionZone")}
                    </span>
                  )}
                  {zones.relegationSlots > 0 && (
                    <span className="flex items-center gap-1.5">
                      <span className="h-2 w-2 rounded-full bg-red-500" />
                      {t("schedule.relegationZone")}
                    </span>
                  )}
                </div>
              )}
            </div>
          </Card>
        ))}

      {/* Fixtures */}
      {view === "fixtures" && (
        <div className="flex flex-col gap-4">
          {sortedMatchdays.map(([md, fixtures]) => (
            <Card key={md}>
              <div className="px-5 py-3 border-b border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 rounded-t-xl">
                <h4 className="font-heading font-bold text-sm uppercase tracking-wider text-gray-600 dark:text-gray-300">
                  {t("schedule.matchday", { number: md })} —{" "}
                  {formatMatchDate(fixtures[0].date)}
                </h4>
              </div>
              <CardBody className="p-0">
                <div className="divide-y divide-gray-100 dark:divide-navy-600">
                  {fixtures.map((f) =>
                    renderFixtureRow(f, `tournaments-fixture-${f.id}`),
                  )}
                </div>
              </CardBody>
            </Card>
          ))}
        </div>
      )}
      {/* Awards */}
      {view === "awards" && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
          {!seasonComplete && (
            <div className="md:col-span-2 lg:col-span-3 rounded-xl border border-amber-300/50 bg-amber-50 px-4 py-3 text-sm text-amber-700 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-300">
              {t("tournaments.awards.currentLeaders")}
            </div>
          )}
          {awards ? (
            <>
              <AwardCard
                icon={<Briefcase className="w-5 h-5 text-accent-500" />}
                title={t("tournaments.awards.managerOfSeasonTitle")}
                subtitle={t("tournaments.awards.managerOfSeasonSubtitle")}
                entries={awards.manager_of_season}
                unit={t("tournaments.awards.units.winRate")}
                emptyText={t("tournaments.awards.noDataYet")}
                decimal={false}
                onSelectTeam={onSelectTeam}
              />
              <AwardCard
                icon={<Zap className="w-5 h-5 text-accent-500" />}
                title={t("tournaments.awards.goldenBootTitle")}
                subtitle={t("tournaments.awards.goldenBootSubtitle")}
                entries={awards.golden_boot}
                unit={t("tournaments.awards.units.goals")}
                emptyText={t("tournaments.awards.noDataYet")}
                onSelectPlayer={onSelectPlayer}
                onSelectTeam={onSelectTeam}
              />
              <AwardCard
                icon={<Star className="w-5 h-5 text-purple-500" />}
                title={t("tournaments.awards.assistKingTitle")}
                subtitle={t("tournaments.awards.assistKingSubtitle")}
                entries={awards.assist_king}
                unit={t("tournaments.awards.units.assists")}
                emptyText={t("tournaments.awards.noDataYet")}
                onSelectPlayer={onSelectPlayer}
                onSelectTeam={onSelectTeam}
              />
              <AwardCard
                icon={<Trophy className="w-5 h-5 text-primary-500" />}
                title={t("tournaments.awards.playerOfYearTitle")}
                subtitle={t("tournaments.awards.playerOfYearSubtitle")}
                entries={awards.player_of_year}
                unit={t("tournaments.awards.units.rating")}
                emptyText={t("tournaments.awards.noDataYet")}
                decimal
                onSelectPlayer={onSelectPlayer}
                onSelectTeam={onSelectTeam}
              />
              <AwardCard
                icon={<Shield className="w-5 h-5 text-blue-500" />}
                title={t("tournaments.awards.goldenGloveTitle")}
                subtitle={t("tournaments.awards.goldenGloveSubtitle")}
                entries={awards.clean_sheet_king}
                unit={t("tournaments.awards.units.cleanSheets")}
                emptyText={t("tournaments.awards.noDataYet")}
                onSelectPlayer={onSelectPlayer}
                onSelectTeam={onSelectTeam}
              />
              <AwardCard
                icon={<Users className="w-5 h-5 text-green-500" />}
                title={t("tournaments.awards.everPresentTitle")}
                subtitle={t("tournaments.awards.everPresentSubtitle")}
                entries={awards.most_appearances}
                unit={t("tournaments.awards.units.apps")}
                emptyText={t("tournaments.awards.noDataYet")}
                onSelectPlayer={onSelectPlayer}
                onSelectTeam={onSelectTeam}
              />
              <AwardCard
                icon={<Star className="w-5 h-5 text-amber-500" />}
                title={t("tournaments.awards.youngPlayerTitle")}
                subtitle={t("tournaments.awards.youngPlayerSubtitle")}
                entries={awards.young_player}
                unit={t("tournaments.awards.units.rating")}
                emptyText={t("tournaments.awards.noDataYet")}
                decimal
                onSelectPlayer={onSelectPlayer}
                onSelectTeam={onSelectTeam}
              />
            </>
          ) : awardsLoadState === "error" ? (
            <div className="col-span-full text-center py-12">
              <Award className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-400 dark:text-gray-500 mb-4">
                {t("tournaments.awards.noDataYet")}
              </p>
              <button
                onClick={() => setAwardsRetryCount((count) => count + 1)}
                className="px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider bg-primary-500 text-white hover:bg-primary-600 transition-colors"
              >
                {t("common.retry")}
              </button>
            </div>
          ) : (
            <div className="col-span-full text-center py-12">
              <Award className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-400 dark:text-gray-500">
                {t("tournaments.loadingAwards")}
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function AwardCard({
  icon,
  title,
  subtitle,
  entries,
  unit,
  emptyText,
  decimal,
  onSelectPlayer,
  onSelectTeam,
}: {
  icon: ReactNode;
  title: string;
  subtitle: string;
  entries: Array<SeasonAwardEntryData | SeasonManagerAwardEntryData>;
  unit: string;
  emptyText: string;
  decimal?: boolean;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam: (id: string) => void;
}) {
  const { t } = useTranslation();
  const buildAwardMenuItems = (entry: SeasonAwardEntryData | SeasonManagerAwardEntryData) => {
    const items = [buildViewTeamMenuItem(t, () => onSelectTeam(entry.team_id))];

    if (typeof onSelectPlayer === "function" && "player_id" in entry) {
      items.unshift(
        buildViewProfileMenuItem(t, () => onSelectPlayer(entry.player_id)),
      );
    }

    return items;
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          {icon}
          <div>
            <span>{title}</span>
            <p className="text-[10px] text-gray-400 dark:text-gray-500 font-normal normal-case tracking-normal">
              {subtitle}
            </p>
          </div>
        </div>
      </CardHeader>
      <CardBody className="p-0">
        {entries.length === 0 ? (
          <p className="p-4 text-sm text-gray-400 dark:text-gray-500 text-center">
            {emptyText}
          </p>
        ) : (
          <div className="divide-y divide-gray-100 dark:divide-navy-600">
            {entries.map((entry, i) => (
              <ContextMenu
                items={buildAwardMenuItems(entry)}
                key={"player_id" in entry ? entry.player_id : entry.manager_id}
              >
                <div
                  className="flex items-center px-4 py-2.5 gap-3"
                  data-testid={`tournaments-award-entry-${"player_id" in entry ? entry.player_id : entry.manager_id}`}
                >
                  <span
                    className={`font-heading font-bold text-sm w-5 text-center ${i === 0
                      ? "text-accent-500"
                      : "text-gray-400 dark:text-gray-500"
                      }`}
                  >
                    {i + 1}
                  </span>
                  <div className="flex-1 min-w-0">
                    <p
                      className={`text-sm font-semibold truncate ${i === 0
                        ? "text-gray-900 dark:text-gray-100"
                        : "text-gray-700 dark:text-gray-300"
                        }`}
                    >
                      {"player_name" in entry ? entry.player_name : entry.manager_name}
                    </p>
                    <p className="text-xs text-gray-400 dark:text-gray-500">
                      {entry.team_name}
                    </p>
                  </div>
                  <span
                    className={`font-heading font-bold tabular-nums ${i === 0
                      ? "text-lg text-accent-500"
                      : "text-sm text-gray-600 dark:text-gray-400"
                      }`}
                  >
                    {decimal ? entry.value.toFixed(2) : `${Math.round("win_rate" in entry ? entry.win_rate : entry.value)}`}
                  </span>
                  <span className="text-[10px] text-gray-400 dark:text-gray-500 w-12">
                    {unit}
                  </span>
                </div>
              </ContextMenu>
            ))}
          </div>
        )}
      </CardBody>
    </Card>
  );
}
