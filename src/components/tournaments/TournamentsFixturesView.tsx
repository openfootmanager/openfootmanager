import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

import TournamentsFixtureRow from "./TournamentsFixtureRow";
import type { TournamentsTeamLookup } from "./teamLookup";
import { formatMatchDate } from "../../lib/helpers";
import { Card, CardBody } from "../ui";
import type { FixtureData } from "../../store/gameStore";

interface TournamentsFixturesViewProps {
  /** Matchdays in playing order, each with its fixtures. */
  sortedMatchdays: Array<[number, FixtureData[]]>;
  teams: TournamentsTeamLookup;
}

/** One date, or the first and last when a round was postponed in part. */
function playedDates(fixtures: FixtureData[], t: TFunction): string {
  const dates = fixtures.map((fixture) => fixture.date).sort();
  const first = dates[0];
  const last = dates[dates.length - 1];
  return first === last
    ? formatMatchDate(first)
    : t("schedule.dateRange", { from: formatMatchDate(first), to: formatMatchDate(last) });
}

/** The competition's fixture list, one card per matchday. */
export default function TournamentsFixturesView({
  sortedMatchdays,
  teams,
}: TournamentsFixturesViewProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-4">
      {sortedMatchdays.map(([matchday, fixtures]) => (
        <Card key={matchday}>
          <div className="px-5 py-3 border-b border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 rounded-t-xl">
            <h4 className="font-heading font-bold text-sm uppercase tracking-wider text-gray-600 dark:text-gray-300">
              {t("schedule.matchday", { number: matchday })} — {playedDates(fixtures, t)}
            </h4>
          </div>
          <CardBody className="p-0">
            <div className="divide-y divide-gray-100 dark:divide-navy-600">
              {fixtures.map((fixture) => (
                <TournamentsFixtureRow
                  key={fixture.id}
                  fixture={fixture}
                  testId={`tournaments-fixture-${fixture.id}`}
                  teams={teams}
                />
              ))}
            </div>
          </CardBody>
        </Card>
      ))}
    </div>
  );
}
