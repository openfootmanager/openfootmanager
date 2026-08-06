import { useState } from "react";
import CompetitionsOverview from "./CompetitionsOverview";
import KnockoutBracket from "./KnockoutBracket";
import TournamentsAwardsGrid from "./TournamentsAwardsGrid";
import {
  buildTopScorers,
  byTablePosition,
  isKnockoutCompetition,
  localizedRoundName,
  summarizeCompetitionProgress,
} from "./TournamentsTab.helpers";
import { useSeasonAwards } from "./useSeasonAwards";
import { useTournamentsData } from "./useTournamentsData";
import {
  FixtureData,
  GameStateData,
  LeagueData,
} from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import StandingsTable from "./StandingsTable";
import { competitionDisplayName } from "../../lib/competitionName";
import { Card, CardHeader, CardBody, Badge, Select } from "../ui";
import {
  Trophy,
  Calendar,
  TableProperties,
  Award,
  GitBranch,
} from "lucide-react";
import {
  getCompetitiveFixtures,
  getPromotionRelegationZones,
  formatMatchDate,
} from "../../lib/helpers";
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

export default function TournamentsTab({
  gameState,
  onSelectTeam,
  onSelectPlayer,
}: TournamentsTabProps) {
  const { t } = useTranslation();
  const {
    teamNames,
    nationalTeamNames,
    nationalTeamNameKeys,
    playerNames,
    userTeamId,
    seasonContext,
    isPreseason,
    activeCompetitions,
    setSelectedCompetitionId,
    league,
    currentSeason,
    worldCupChampion,
  } = useTournamentsData(gameState);

  const [view, setView] = useState<
    "overview" | "fixtures" | "standings" | "awards"
  >("overview");
  const { awards, awardsLoadState, retryAwards } = useSeasonAwards(
    currentSeason,
    view === "awards",
    gameState.clock?.current_date,
  );

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

  const {
    sortedMatchdays,
    completedMatchdays,
    totalMatchdays,
    seasonComplete,
    totalGoals,
    completedMatches,
  } = summarizeCompetitionProgress(competitiveFixtures);

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

  const topScorers = buildTopScorers(competitiveFixtures, resolvedPlayerNames);

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
                <StandingsTable
                  standings={standings}
                  variant="compact"
                  userTeamId={userTeamId}
                  resolveTeamName={resolveTeamName}
                  onSelectTeam={onSelectTeam}
                  testIdPrefix="tournaments-overview-standing"
                />
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
              <StandingsTable
                standings={standings}
                variant="full"
                userTeamId={userTeamId}
                resolveTeamName={resolveTeamName}
                onSelectTeam={onSelectTeam}
                testIdPrefix="tournaments-standing"
                zones={zones}
              />
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
        <TournamentsAwardsGrid
          awards={awards}
          awardsLoadState={awardsLoadState}
          retryAwards={retryAwards}
          seasonComplete={seasonComplete}
          onSelectTeam={onSelectTeam}
          onSelectPlayer={onSelectPlayer}
        />
      )}
    </div>
  );
}
