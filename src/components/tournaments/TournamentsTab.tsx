import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GameStateData, FixtureData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import { Card, CardHeader, CardBody, Badge } from "../ui";
import {
  Trophy,
  Calendar,
  TableProperties,
  Award,
  Star,
  Shield,
  Users,
  Zap,
} from "lucide-react";
import {
  getCompetitiveFixtures,
  getTeamName,
  formatMatchDate,
} from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { useTranslation } from "react-i18next";
import {
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";

interface AwardEntry {
  player_id: string;
  player_name: string;
  team_id: string;
  team_name: string;
  value: number;
}
interface SeasonAwards {
  golden_boot: AwardEntry[];
  assist_king: AwardEntry[];
  player_of_year: AwardEntry[];
  clean_sheet_king: AwardEntry[];
  most_appearances: AwardEntry[];
  young_player: AwardEntry[];
}

interface TournamentsTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
  onSelectPlayer?: (id: string) => void;
}

export default function TournamentsTab({
  gameState,
  onSelectTeam,
  onSelectPlayer,
}: TournamentsTabProps) {
  const { t } = useTranslation();
  const league = gameState.league;
  const userTeamId = gameState.manager.team_id;
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";
  const [view, setView] = useState<
    "overview" | "fixtures" | "standings" | "awards"
  >("overview");
  const [awardsBySeason, setAwardsBySeason] = useState<
    Record<number, SeasonAwards>
  >({});
  const [awardsLoadState, setAwardsLoadState] = useState<
    "idle" | "loading" | "error"
  >("idle");
  const [awardsRetryCount, setAwardsRetryCount] = useState(0);
  const currentSeason = league?.season ?? 0;
  const awards = awardsBySeason[currentSeason] ?? null;

  useEffect(() => {
    if (view !== "awards" || awards) {
      return;
    }

    let cancelled = false;
    setAwardsLoadState("loading");

    invoke<SeasonAwards>("get_season_awards")
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

  const standings = [...league.standings].sort(
    (a, b) =>
      b.points - a.points ||
      b.goals_for - b.goals_against - (a.goals_for - a.goals_against) ||
      b.goals_for - a.goals_for,
  );

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
  const totalGoals = competitiveFixtures
    .filter((f) => f.result)
    .reduce((s, f) => s + (f.result!.home_goals + f.result!.away_goals), 0);
  const completedMatches = competitiveFixtures.filter(
    (f) => f.status === "Completed",
  ).length;

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
        player: gameState.players.find((p) => p.id === pid),
        goals: g,
      }))
      .filter((e) => e.player)
      .sort((a, b) => b.goals - a.goals)
      .slice(0, 10);
  })();

  const buildFixtureMenuItems = (fixture: FixtureData) => [
    {
      ...buildViewTeamMenuItem(t, () => onSelectTeam(fixture.home_team_id)),
      label: `${t("common.viewTeam")}: ${getTeamName(gameState.teams, fixture.home_team_id)}`,
    },
    {
      ...buildViewTeamMenuItem(t, () => onSelectTeam(fixture.away_team_id)),
      label: `${t("common.viewTeam")}: ${getTeamName(gameState.teams, fixture.away_team_id)}`,
    },
  ];

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

  return (
    <div className="max-w-6xl mx-auto">
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
          <div className="flex items-center gap-4">
            <div className="w-14 h-14 rounded-xl bg-accent-500/20 flex items-center justify-center">
              <Trophy className="w-7 h-7 text-accent-400" />
            </div>
            <div className="flex-1">
              <h2 className="text-2xl font-heading font-bold text-white uppercase tracking-wide">
                {league.name}
              </h2>
              <p className="text-gray-400 text-sm mt-0.5">
                {t("schedule.season", { number: league.season })} —{" "}
                {t("tournaments.nTeams", { count: league.standings.length })}
              </p>
            </div>
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
              <>
                <TableProperties className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                {t("schedule.standings")}
              </>
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
          {/* Mini standings */}
          <Card className="lg:col-span-2">
            <CardHeader>{t("tournaments.leagueTable")}</CardHeader>
            <CardBody className="p-0">
              {isPreseason ? (
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
                              {getTeamName(gameState.teams, entry.team_id)}
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
                        entry.player!.id,
                        entry.player!.team_id,
                      )}
                      key={entry.player!.id}
                    >
                      <div
                        className="flex items-center px-4 py-2.5 gap-3"
                        data-testid={`tournaments-top-scorer-${entry.player!.id}`}
                      >
                        <span className="font-heading font-bold text-sm text-gray-400 dark:text-gray-500 w-5 text-center">
                          {i + 1}
                        </span>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
                            {entry.player!.full_name}
                          </p>
                          <p className="text-xs text-gray-400 dark:text-gray-500">
                            {getTeamName(
                              gameState.teams,
                              entry.player!.team_id ?? "",
                            )}
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

      {/* Full standings */}
      {view === "standings" &&
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
                {league.name} —{" "}
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
                          <td className="py-3 px-4 font-heading font-bold text-sm text-gray-400">
                            {idx + 1}
                          </td>
                          <td
                            className={`py-3 px-4 font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                          >
                            {getTeamName(gameState.teams, entry.team_id)}
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
                  {fixtures.map((f) => {
                    const isUserMatch =
                      f.home_team_id === userTeamId ||
                      f.away_team_id === userTeamId;
                    const completed = f.status === "Completed";
                    return (
                      <ContextMenu items={buildFixtureMenuItems(f)} key={f.id}>
                        <div
                          className={`flex items-center px-5 py-3 transition-colors ${isUserMatch ? "bg-primary-50/50 dark:bg-primary-500/5" : ""}`}
                          data-testid={`tournaments-fixture-${f.id}`}
                        >
                          <span
                            onClick={() => onSelectTeam(f.home_team_id)}
                            className={`flex-1 text-right font-semibold text-sm cursor-pointer hover:underline ${f.home_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                          >
                            {getTeamName(gameState.teams, f.home_team_id)}
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
                            onClick={() => onSelectTeam(f.away_team_id)}
                            className={`flex-1 text-left font-semibold text-sm cursor-pointer hover:underline ${f.away_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                          >
                            {getTeamName(gameState.teams, f.away_team_id)}
                          </span>
                        </div>
                      </ContextMenu>
                    );
                  })}
                </div>
              </CardBody>
            </Card>
          ))}
        </div>
      )}
      {/* Awards */}
      {view === "awards" && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
          {awards ? (
            <>
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
                Retry
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
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  entries: AwardEntry[];
  unit: string;
  emptyText: string;
  decimal?: boolean;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam: (id: string) => void;
}) {
  const { t } = useTranslation();
  const buildAwardMenuItems = (entry: AwardEntry) => {
    const items = [buildViewTeamMenuItem(t, () => onSelectTeam(entry.team_id))];

    if (typeof onSelectPlayer === "function") {
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
                key={entry.player_id}
              >
                <div
                  className="flex items-center px-4 py-2.5 gap-3"
                  data-testid={`tournaments-award-entry-${entry.player_id}`}
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
                      {entry.player_name}
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
                    {decimal ? entry.value.toFixed(2) : entry.value}
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
