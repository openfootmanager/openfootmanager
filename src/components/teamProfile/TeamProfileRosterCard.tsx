import { countryName } from "../../lib/countries";
import {
  calcAge,
  calcOvr,
  formatVal,
  positionBadgeVariant,
} from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { Badge, Card, CardBody, CardHeader, CountryFlag, ProgressBar } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import type { TeamProfileTranslate } from "./TeamProfile.types";

interface TeamProfileRosterCardProps {
  currentDate: string;
  roster: PlayerData[];
  isOwnTeam: boolean;
  locale: string;
  t: TeamProfileTranslate;
  onSelectPlayer?: (id: string) => void;
}

export default function TeamProfileRosterCard({
  currentDate,
  roster,
  isOwnTeam,
  locale,
  t,
  onSelectPlayer,
}: TeamProfileRosterCardProps) {
  return (
    <Card className="lg:col-span-3">
      <CardHeader>
        {t("teams.squad")} ({roster.length})
      </CardHeader>
      <CardBody className="p-0">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.position")}
                </th>
                <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.name")}
                </th>
                <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.age")}
                </th>
                <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.nationality")}
                </th>
                <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.value")}
                </th>
                {isOwnTeam && (
                  <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.condition")}
                  </th>
                )}
                <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.ovr")}
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
              {roster.map((player) => {
                const ovr = calcOvr(
                  player,
                  player.natural_position || player.position,
                );
                const age = calcAge(player.date_of_birth, currentDate);

                return (
                  <tr
                    key={player.id}
                    onClick={() => onSelectPlayer?.(player.id)}
                    className="hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors cursor-pointer group"
                  >
                    <td className="py-3 px-5">
                      <Badge
                        variant={positionBadgeVariant(
                          player.natural_position || player.position,
                        )}
                      >
                        {translatePositionAbbreviation(
                          t,
                          player.natural_position || player.position,
                        )}
                      </Badge>
                    </td>
                    <td className="py-3 px-5">
                      <span className="font-semibold text-sm text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                        {player.full_name}
                      </span>
                    </td>
                    <td className="py-3 px-5 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                      {age}
                    </td>
                    <td className="py-3 px-5 text-sm text-gray-500 dark:text-gray-400">
                      <div className="flex items-center gap-1">
                        <CountryFlag
                          code={player.nationality}
                          locale={locale}
                          className="text-lg leading-none"
                        />
                        <span>{countryName(player.nationality, locale)}</span>
                      </div>
                    </td>
                    <td className="py-3 px-5 text-sm text-gray-600 dark:text-gray-400">
                      {formatVal(player.market_value)}
                    </td>
                    {isOwnTeam && (
                      <td className="py-3 px-5">
                        <ProgressBar
                          value={player.condition}
                          variant="auto"
                          size="sm"
                          showLabel
                          className="max-w-[100px]"
                        />
                      </td>
                    )}
                    <td className="py-3 px-5">
                      <span
                        className={`font-heading font-bold text-lg tabular-nums ${isOwnTeam
                            ? ovr >= 75
                              ? "text-primary-500"
                              : ovr >= 55
                                ? "text-accent-500"
                                : "text-gray-400"
                            : "text-gray-400"
                          }`}
                      >
                        {isOwnTeam ? ovr : "??"}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </CardBody>
    </Card>
  );
}
