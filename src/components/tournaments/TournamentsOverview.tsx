import { useTranslation } from "react-i18next";

import StandingsTable from "./StandingsTable";
import TournamentsGroupTable from "./TournamentsGroupTable";
import TournamentsPreseasonNote from "./TournamentsPreseasonNote";
import TournamentsTopScorers from "./TournamentsTopScorers";
import { localizedRoundName, type TopScorerEntry } from "./TournamentsTab.helpers";
import type { TournamentsTeamLookup } from "./teamLookup";
import { Card, CardHeader, CardBody, Badge } from "../ui";
import type { LeagueData, StandingData } from "../../store/gameStore";

type Group = NonNullable<LeagueData["groups"]>[number];
type KnockoutRound = NonNullable<LeagueData["knockout_rounds"]>[number];

interface TournamentsOverviewProps {
  standings: StandingData[];
  groups: Group[];
  knockoutRounds: KnockoutRound[];
  isKnockout: boolean;
  isPreseason: boolean;
  topScorers: TopScorerEntry[];
  teams: TournamentsTeamLookup;
  onSelectPlayer?: (id: string) => void;
}

/**
 * The competition at a glance: how it stands, and who is scoring.
 *
 * What "how it stands" means depends on the competition — a bracket's round
 * progress, a group stage's mini tables, or a league's own table — and during
 * preseason it means none of those, because nothing has been played yet.
 */
export default function TournamentsOverview({
  standings,
  groups,
  knockoutRounds,
  isKnockout,
  isPreseason,
  topScorers,
  teams,
  onSelectPlayer,
}: TournamentsOverviewProps) {
  const { t } = useTranslation();

  const groupGrid = (
    <div className="grid grid-cols-1 md:grid-cols-2">
      {groups.map((group) => (
        <TournamentsGroupTable key={group.id} group={group} teams={teams} />
      ))}
    </div>
  );

  const roundProgress = (
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
  );

  // Same order as TournamentsStandingsView: what a competition has to show
  // depends on its shape, and a drawn bracket outranks the group stage feeding it.
  const summary = (() => {
    if (isKnockout) {
      return knockoutRounds.length === 0 && groups.length > 0
        ? groupGrid
        : roundProgress;
    }
    if (groups.length > 0) return groupGrid;
    if (isPreseason) return <TournamentsPreseasonNote />;
    return (
      <StandingsTable
        standings={standings}
        variant="compact"
        teams={teams}
        testIdPrefix="tournaments-overview-standing"
      />
    );
  })();

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
      <Card className="lg:col-span-2">
        <CardHeader>
          {isKnockout ? t("tournaments.bracket") : t("tournaments.leagueTable")}
        </CardHeader>
        <CardBody className="p-0">{summary}</CardBody>
      </Card>

      <TournamentsTopScorers
        topScorers={topScorers}
        onSelectTeam={teams.onSelectTeam}
        onSelectPlayer={onSelectPlayer}
      />
    </div>
  );
}
