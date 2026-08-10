import { useState } from "react";
import CompetitionsOverview from "./CompetitionsOverview";
import TournamentsAwardsGrid from "./TournamentsAwardsGrid";
import {
  buildTopScorers,
  byTablePosition,
  isKnockoutCompetition,
  summarizeCompetitionProgress,
} from "./TournamentsTab.helpers";
import { useSeasonAwards } from "./useSeasonAwards";
import { useTournamentsData } from "./useTournamentsData";
import { GameStateData } from "../../store/gameStore";
import TournamentsFixtureRow from "./TournamentsFixtureRow";
import TournamentsLeagueHeader from "./TournamentsLeagueHeader";
import TournamentsOverview from "./TournamentsOverview";
import TournamentsStandingsView from "./TournamentsStandingsView";
import TournamentsViewTabs, {
  type TournamentsView,
} from "./TournamentsViewTabs";
import type { TournamentsTeamLookup } from "./teamLookup";
import { nationalTeamDisplayName } from "../../lib/nationalTeams";
import { Card, CardBody, Badge } from "../ui";
import { Trophy } from "lucide-react";
import {
  getCompetitiveFixtures,
  getPromotionRelegationZones,
  formatMatchDate,
} from "../../lib/helpers";
import { useTranslation } from "react-i18next";

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

  const [view, setView] = useState<TournamentsView>("overview");
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

  const progress = summarizeCompetitionProgress(competitiveFixtures);
  const { sortedMatchdays, seasonComplete } = progress;

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
    return nationalTeamDisplayName(
      nationalTeamNameKeys[id],
      nationalTeamNames[id] ?? id,
      t,
    );
  };

  const teams: TournamentsTeamLookup = {
    userTeamId,
    isClubTeam,
    resolveTeamName,
    onSelectTeam,
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

      <TournamentsLeagueHeader
        league={league}
        activeCompetitions={activeCompetitions}
        onSelectCompetition={setSelectedCompetitionId}
        participantCount={participantCount}
        progress={progress}
        worldCupChampion={worldCupChampion}
      />

      <TournamentsViewTabs
        view={view}
        onSelectView={setView}
        isKnockout={isKnockout}
      />

      {view === "overview" && (
        <TournamentsOverview
          standings={standings}
          groups={groups}
          knockoutRounds={knockoutRounds}
          isKnockout={isKnockout}
          isPreseason={isPreseason}
          topScorers={topScorers}
          teams={teams}
          onSelectPlayer={onSelectPlayer}
        />
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

      {view === "standings" && (
        <TournamentsStandingsView
          league={league}
          standings={standings}
          groups={groups}
          knockoutRounds={knockoutRounds}
          isKnockout={isKnockout}
          isPreseason={isPreseason}
          zones={zones}
          teams={teams}
        />
      )}

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
                  {fixtures.map((f) => (
                    <TournamentsFixtureRow
                      key={f.id}
                      fixture={f}
                      testId={`tournaments-fixture-${f.id}`}
                      teams={teams}
                    />
                  ))}
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
