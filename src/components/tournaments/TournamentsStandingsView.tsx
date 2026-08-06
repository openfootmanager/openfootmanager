import { Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";

import KnockoutBracket from "./KnockoutBracket";
import StandingsTable from "./StandingsTable";
import TournamentsGroupTable from "./TournamentsGroupTable";
import { localizedRoundName } from "./TournamentsTab.helpers";
import type { TournamentsTeamLookup } from "./teamLookup";
import { competitionDisplayName } from "../../lib/competitionName";
import { Card, CardBody } from "../ui";
import type { LeagueData, StandingData } from "../../store/gameStore";

type Group = NonNullable<LeagueData["groups"]>[number];
type KnockoutRound = NonNullable<LeagueData["knockout_rounds"]>[number];

interface TournamentsStandingsViewProps {
  league: LeagueData;
  standings: StandingData[];
  groups: Group[];
  knockoutRounds: KnockoutRound[];
  isKnockout: boolean;
  isPreseason: boolean;
  zones: { promotionSlots: number; relegationSlots: number };
  teams: TournamentsTeamLookup;
}

/**
 * How a competition stands, in whichever shape it has.
 *
 * A cup shows its bracket, and its group stage above it while one is running.
 * A qualifying competition is groups alone. Everything else is a league table.
 */
export default function TournamentsStandingsView({
  league,
  standings,
  groups,
  knockoutRounds,
  isKnockout,
  isPreseason,
  zones,
  teams,
}: TournamentsStandingsViewProps) {
  const { t } = useTranslation();

  const groupGrid = (
    <Card>
      <CardBody className="p-0">
        <div className="grid grid-cols-1 md:grid-cols-2">
          {groups.map((group) => (
            <TournamentsGroupTable key={group.id} group={group} teams={teams} />
          ))}
        </div>
      </CardBody>
    </Card>
  );

  if (isKnockout) {
    return (
      <div className="flex flex-col gap-4">
        {groups.length > 0 && groupGrid}
        {knockoutRounds.length > 0 && (
          <KnockoutBracket
            rounds={knockoutRounds}
            fixtures={league.fixtures}
            resolveTeamName={teams.resolveTeamName}
            localizedRoundName={(name) => localizedRoundName(t, name)}
            userTeamId={teams.userTeamId}
            roundCompleteLabel={t("tournaments.roundComplete")}
            roundInProgressLabel={t("tournaments.roundInProgress")}
            byeLabel={t("tournaments.bye")}
            tbdLabel={t("tournaments.tbd")}
          />
        )}
      </div>
    );
  }

  // Group tables for non-knockout competitions, e.g. World Cup qualifying.
  if (groups.length > 0) {
    return groupGrid;
  }

  if (isPreseason) {
    return (
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
    );
  }

  return (
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
          teams={teams}
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
  );
}
