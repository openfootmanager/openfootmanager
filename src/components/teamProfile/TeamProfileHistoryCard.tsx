import type { TeamData } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";
import type { TeamProfileTranslate } from "./TeamProfile.types";
import { TeamSeasonHistoryChart } from "./TeamSeasonHistoryChart";

interface TeamProfileHistoryCardProps {
  history: TeamData["history"];
  t: TeamProfileTranslate;
}

export default function TeamProfileHistoryCard({
  history,
  t,
}: TeamProfileHistoryCardProps) {
  if (history.length === 0) {
    return null;
  }

  return (
    <Card className="lg:col-span-3">
      <CardHeader>{t("teamProfile.seasonHistory")}</CardHeader>
      <CardBody className="p-0">
        <div className="px-4 pt-4 pb-2">
          <TeamSeasonHistoryChart
            history={history}
            wonLabel={t("common.won")}
            drawnLabel={t("common.drawn")}
            lostLabel={t("common.lost")}
            positionLabel={t("common.position")}
          />
        </div>
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("schedule.season", { number: "" })}
              </th>
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                {t("common.position")}
              </th>
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                {t("common.played")}
              </th>
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                {t("common.won")}
              </th>
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                {t("common.drawn")}
              </th>
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                {t("common.lost")}
              </th>
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                {t("common.gf")}
              </th>
              <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                {t("common.ga")}
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
            {history.map((record, index) => (
              <tr key={index}>
                <td className="py-3 px-5 font-semibold text-sm text-gray-800 dark:text-gray-200">
                  {record.season}/{record.season + 1}
                </td>
                <td className="py-3 px-5 text-center font-heading font-bold text-sm text-primary-500">
                  #{record.league_position}
                </td>
                <td className="py-3 px-5 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                  {record.played}
                </td>
                <td className="py-3 px-5 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                  {record.won}
                </td>
                <td className="py-3 px-5 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                  {record.drawn}
                </td>
                <td className="py-3 px-5 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                  {record.lost}
                </td>
                <td className="py-3 px-5 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                  {record.goals_for}
                </td>
                <td className="py-3 px-5 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                  {record.goals_against}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </CardBody>
    </Card>
  );
}
