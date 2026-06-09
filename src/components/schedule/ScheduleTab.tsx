import { useEffect, useState } from "react";
import { FixtureData, GameStateData, LeagueData } from "../../store/gameStore";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import { Badge, Card, CardBody } from "../ui";
import {
  Calendar as CalendarIcon,
  TableProperties,
  Trophy,
} from "lucide-react";
import {
  formatMatchDate,
  getActiveCompetitions,
  getAllFixturesAcrossCompetitions,
  getTeamName,
} from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { useTranslation } from "react-i18next";

interface ScheduleTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

function sortStandings(competition: LeagueData | null): LeagueData["standings"] {
  if (!competition) {
    return [];
  }

  return [...competition.standings].sort(
    (a, b) =>
      b.points - a.points ||
      b.goals_for - b.goals_against - (a.goals_for - a.goals_against) ||
      b.goals_for - a.goals_for,
  );
}

export default function ScheduleTab({
  gameState,
  onSelectTeam,
}: ScheduleTabProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<"fixtures" | "standings">("fixtures");
  const [selectedCompetitionId, setSelectedCompetitionId] = useState<string | null>(null);
  const userTeamId = gameState.manager.team_id;
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";
  const activeCompetitions = getActiveCompetitions(gameState);
  const userCompetitions = activeCompetitions.filter((competition) =>
    competition.participant_ids?.includes(userTeamId ?? ""),
  );
  const selectedCompetition =
    activeCompetitions.find((competition) => competition.id === selectedCompetitionId) ??
    userCompetitions.find((competition) => competition.standings.length > 0) ??
    activeCompetitions.find((competition) => competition.standings.length > 0) ??
    userCompetitions[0] ??
    activeCompetitions[0] ??
    null;
  const standings = sortStandings(selectedCompetition);
  const competitionNames = new Map(
    activeCompetitions.map((competition) => [competition.id, competition.name]),
  );

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

    const preferredCompetitionId =
      userCompetitions.find((competition) => competition.standings.length > 0)?.id ??
      activeCompetitions.find((competition) => competition.standings.length > 0)?.id ??
      userCompetitions[0]?.id ??
      activeCompetitions[0].id;
    setSelectedCompetitionId(preferredCompetitionId);
  }, [activeCompetitions, selectedCompetitionId, userCompetitions]);

  const buildTeamMenuItem = (
    label: string,
    teamId: string,
  ): ContextMenuItem => ({
    label,
    onClick: () => onSelectTeam(teamId),
  });

  if (activeCompetitions.length === 0) {
    return (
      <p className="py-8 text-center text-gray-500 dark:text-gray-400">
        {t("schedule.noLeague")}
      </p>
    );
  }

  const getFixtureGroupKey = (fixture: FixtureData): string => {
    if (fixture.competition === "League") {
      return `${fixture.competition_id}-league-${fixture.matchday}`;
    }

    return `${fixture.competition_id}-${fixture.competition}-${fixture.date}`;
  };

  const getFixtureGroupLabel = (fixture: FixtureData): string => {
    const competitionId = fixture.competition_id ?? "";
    const competitionName =
      competitionNames.get(competitionId) ??
      fixture.competition_id ??
      t("schedule.fixtures");

    if (fixture.competition === "League") {
      return `${competitionName} - ${t("schedule.matchday", { number: fixture.matchday })} - ${formatMatchDate(fixture.date)}`;
    }

    if (fixture.competition === "PreseasonTournament") {
      return `${competitionName} - ${t("season.preseasonTournament")} - ${formatMatchDate(fixture.date)}`;
    }

    return `${competitionName} - ${formatMatchDate(fixture.date)}`;
  };

  const matchdays = new Map<string, FixtureData[]>();
  getAllFixturesAcrossCompetitions(gameState).forEach((fixture) => {
    const key = getFixtureGroupKey(fixture);
    const list = matchdays.get(key) || [];
    list.push(fixture);
    matchdays.set(key, list);
  });
  const sortedMatchdays = Array.from(matchdays.entries()).sort((a, b) => {
    const leftFixture = a[1][0];
    const rightFixture = b[1][0];
    return (
      leftFixture.date.localeCompare(rightFixture.date) ||
      leftFixture.matchday - rightFixture.matchday
    );
  });

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
                {t("season.standingsLocked")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      <div className="mb-5 flex gap-2">
        <button
          onClick={() => setView("fixtures")}
          className={`rounded-lg px-4 py-2 font-heading text-sm font-bold uppercase tracking-wider transition-all ${
            view === "fixtures"
              ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
              : "border border-gray-200 bg-white text-gray-500 hover:text-gray-700 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-400 dark:hover:text-gray-200"
          }`}
        >
          <CalendarIcon className="mr-1.5 inline h-4 w-4 -mt-0.5" />{" "}
          {t("schedule.fixtures")}
        </button>
        <button
          onClick={() => setView("standings")}
          className={`rounded-lg px-4 py-2 font-heading text-sm font-bold uppercase tracking-wider transition-all ${
            view === "standings"
              ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
              : "border border-gray-200 bg-white text-gray-500 hover:text-gray-700 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-400 dark:hover:text-gray-200"
          }`}
        >
          <TableProperties className="mr-1.5 inline h-4 w-4 -mt-0.5" />{" "}
          {t("schedule.standings")}
        </button>
        {activeCompetitions.length > 1 && (
          <select
            value={selectedCompetition?.id ?? ""}
            onChange={(event) => setSelectedCompetitionId(event.target.value)}
            className="ml-auto rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-700 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-200"
          >
            {activeCompetitions.map((competition) => (
              <option key={competition.id} value={competition.id}>
                {competition.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {view === "fixtures" && (
        <div className="flex flex-col gap-4">
          {sortedMatchdays.map(([groupKey, fixtures]) => (
            <Card key={groupKey}>
              <div className="rounded-t-xl border-b border-gray-100 bg-gray-50 px-5 py-3 dark:border-navy-600 dark:bg-navy-800">
                <h4 className="font-heading text-sm font-bold uppercase tracking-wider text-gray-600 dark:text-gray-300">
                  {getFixtureGroupLabel(fixtures[0])}
                </h4>
              </div>
              <CardBody className="p-0">
                <div className="divide-y divide-gray-100 dark:divide-navy-600">
                  {fixtures.map((fixture) => {
                    const isUserMatch =
                      fixture.home_team_id === userTeamId ||
                      fixture.away_team_id === userTeamId;
                    const completed = fixture.status === "Completed";
                    const contextItems = [
                      buildTeamMenuItem(
                        `${t("common.viewTeam")}: ${getTeamName(gameState.teams, fixture.home_team_id)}`,
                        fixture.home_team_id,
                      ),
                      buildTeamMenuItem(
                        `${t("common.viewTeam")}: ${getTeamName(gameState.teams, fixture.away_team_id)}`,
                        fixture.away_team_id,
                      ),
                    ];

                    return (
                      <ContextMenu items={contextItems} key={fixture.id}>
                        <div
                          className={`flex items-center px-5 py-3 transition-colors ${
                            isUserMatch ? "bg-primary-50/50 dark:bg-primary-500/5" : ""
                          }`}
                          data-testid={`schedule-fixture-${fixture.id}`}
                        >
                          <span
                            onClick={() => onSelectTeam(fixture.home_team_id)}
                            className={`flex-1 cursor-pointer text-right text-sm font-semibold hover:underline ${
                              fixture.home_team_id === userTeamId
                                ? "text-primary-600 dark:text-primary-400"
                                : "text-gray-800 dark:text-gray-200"
                            }`}
                          >
                            {getTeamName(gameState.teams, fixture.home_team_id)}
                          </span>
                          <div className="mx-3 w-24 text-center">
                            {completed && fixture.result ? (
                              <span className="font-heading text-lg font-bold text-gray-800 dark:text-gray-100">
                                {fixture.result.home_goals} - {fixture.result.away_goals}
                              </span>
                            ) : (
                              <Badge variant="neutral" size="sm">
                                vs
                              </Badge>
                            )}
                          </div>
                          <span
                            onClick={() => onSelectTeam(fixture.away_team_id)}
                            className={`flex-1 cursor-pointer text-left text-sm font-semibold hover:underline ${
                              fixture.away_team_id === userTeamId
                                ? "text-primary-600 dark:text-primary-400"
                                : "text-gray-800 dark:text-gray-200"
                            }`}
                          >
                            {getTeamName(gameState.teams, fixture.away_team_id)}
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

      {view === "standings" &&
        (isPreseason ? (
          <Card>
            <CardBody>
              <div className="flex flex-col items-center gap-2 py-6 text-center">
                <Trophy className="h-8 w-8 text-gray-300 dark:text-navy-600" />
                <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                  {t("season.standingsLocked")}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {seasonContext.season_start
                    ? t("season.startsOn", {
                        date: formatMatchDate(seasonContext.season_start),
                      })
                    : t("season.noOpener")}
                </p>
              </div>
            </CardBody>
          </Card>
        ) : (
          <Card>
            <div className="rounded-t-xl border-b border-gray-100 bg-gradient-to-r from-navy-700 to-navy-800 p-5 dark:border-navy-600">
              <h3 className="flex items-center gap-2 font-heading text-lg font-bold uppercase tracking-wide text-white">
                <Trophy className="h-5 w-5 text-accent-400" />
                {selectedCompetition?.name ?? t("schedule.fixtures")} -{" "}
                {t("schedule.season", { number: selectedCompetition?.season ?? 0 })}
              </h3>
            </div>
            {standings.length === 0 ? (
              <CardBody>
                <p className="py-6 text-center text-sm text-gray-500 dark:text-gray-400">
                  {t("schedule.standingsUnavailable")}
                </p>
              </CardBody>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full border-collapse text-left">
                  <thead>
                    <tr className="border-b border-gray-200 bg-gray-50 text-xs dark:border-navy-600 dark:bg-navy-800">
                      <th className="w-8 px-4 py-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        #
                      </th>
                      <th className="px-4 py-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.team")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.played")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.won")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.drawn")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.lost")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.gf")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.ga")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.gd")}
                      </th>
                      <th className="px-4 py-3 text-center font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.pts")}
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                    {standings.map((entry, index) => {
                      const isUser = entry.team_id === userTeamId;
                      const gd = entry.goals_for - entry.goals_against;
                      const contextItems = [
                        buildTeamMenuItem(t("common.viewTeam"), entry.team_id),
                      ];

                      return (
                        <ContextMenu items={contextItems} key={entry.team_id}>
                          <tr
                            className={`transition-colors ${
                              isUser
                                ? "bg-primary-50 dark:bg-primary-500/10"
                                : "hover:bg-gray-50 dark:hover:bg-navy-700/50"
                            }`}
                            data-testid={`schedule-standings-row-${entry.team_id}`}
                          >
                            <td className="px-4 py-3 font-heading text-sm font-bold text-gray-400 dark:text-gray-500">
                              {index + 1}
                            </td>
                            <td
                              onClick={() => onSelectTeam(entry.team_id)}
                              className={`cursor-pointer px-4 py-3 text-sm font-semibold hover:underline ${
                                isUser
                                  ? "text-primary-600 dark:text-primary-400"
                                  : "text-gray-800 dark:text-gray-200"
                              }`}
                            >
                              {getTeamName(gameState.teams, entry.team_id)}
                            </td>
                            <td className="px-4 py-3 text-center text-sm tabular-nums text-gray-600 dark:text-gray-400">
                              {entry.played}
                            </td>
                            <td className="px-4 py-3 text-center text-sm tabular-nums text-gray-600 dark:text-gray-400">
                              {entry.won}
                            </td>
                            <td className="px-4 py-3 text-center text-sm tabular-nums text-gray-600 dark:text-gray-400">
                              {entry.drawn}
                            </td>
                            <td className="px-4 py-3 text-center text-sm tabular-nums text-gray-600 dark:text-gray-400">
                              {entry.lost}
                            </td>
                            <td className="px-4 py-3 text-center text-sm tabular-nums text-gray-600 dark:text-gray-400">
                              {entry.goals_for}
                            </td>
                            <td className="px-4 py-3 text-center text-sm tabular-nums text-gray-600 dark:text-gray-400">
                              {entry.goals_against}
                            </td>
                            <td
                              className={`px-4 py-3 text-center text-sm font-semibold tabular-nums ${
                                gd > 0
                                  ? "text-primary-500"
                                  : gd < 0
                                    ? "text-red-500"
                                    : "text-gray-500 dark:text-gray-400"
                              }`}
                            >
                              {gd > 0 ? `+${gd}` : gd}
                            </td>
                            <td className="px-4 py-3 text-center font-heading text-sm font-bold tabular-nums text-gray-800 dark:text-gray-100">
                              {entry.points}
                            </td>
                          </tr>
                        </ContextMenu>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </Card>
        ))}
    </div>
  );
}
