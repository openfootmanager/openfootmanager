import { useRatingTiers } from "../../hooks/useRatingTiers";
import { ratingTierClass } from "../../lib/ratingTier";
import { countryName } from "../../lib/countries";
import { calcAge, formatVal, getPlayerOvr } from "../../lib/helpers";
import { knowsPlayerRating, visibleRatingText } from "../../lib/playerKnowledge";
import type { PlayerKnowledge } from "../../services/scoutingService";
import type { PlayerData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import { buildViewProfileMenuItem } from "../playerActions/playerContextMenuItems";
import { Card, CardBody, CardHeader, CountryFlag, PlayerAvatar, ProgressBar } from "../ui";
import type { TeamProfileTranslate } from "./TeamProfile.types";
import { naturalPositionOf } from "../../lib/positions";
import PlayerPositionBadge from "../PlayerPositionBadge";

interface TeamProfileRosterCardProps {
  roster: PlayerData[];
  /** What the club knows about each player, from the backend's one rule. */
  knowledgeOf: (playerId: string) => PlayerKnowledge;
  isOwnTeam: boolean;
  locale: string;
  t: TeamProfileTranslate;
  onSelectPlayer?: (id: string) => void;
}

export default function TeamProfileRosterCard({
  roster,
  knowledgeOf,
  isOwnTeam,
  locale,
  t,
  onSelectPlayer,
}: TeamProfileRosterCardProps) {
  const tierOf = useRatingTiers();
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
                // The rating as the club may see it: its own player's, or a division rival's.
                const ovr = knowsPlayerRating(knowledgeOf(player.id)) ? getPlayerOvr(player) : null;
                const age = calcAge(player.date_of_birth);
                const contextItems = onSelectPlayer
                  ? [buildViewProfileMenuItem(t, () => onSelectPlayer(player.id))]
                  : [];
                const playerRow = (
                  <tr
                    key={player.id}
                    data-testid={`team-profile-roster-${player.id}`}
                    onClick={() => onSelectPlayer?.(player.id)}
                    className={`group transition-colors ${
                      onSelectPlayer
                        ? "hover:bg-gray-50 dark:hover:bg-navy-700/50 cursor-pointer"
                        : ""
                    }`}
                  >
                    <td className="py-3 px-5">
                      <PlayerPositionBadge position={naturalPositionOf(player)} />
                    </td>
                    <td className="py-3 px-5">
                      <div className="flex items-center gap-3 min-w-0">
                        <PlayerAvatar player={player} />
                        <span className="block truncate font-semibold text-sm text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                          {player.full_name}
                        </span>
                      </div>
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
                        className={`font-heading font-bold text-lg tabular-nums ${ratingTierClass(
                          ovr === null ? "Rest" : tierOf(ovr),
                          "text",
                        )}`}
                      >
                        {visibleRatingText(ovr)}
                      </span>
                    </td>
                  </tr>
                );

                if (contextItems.length > 0) {
                  return (
                    <ContextMenu items={contextItems} key={player.id}>
                      {playerRow}
                    </ContextMenu>
                  );
                }

                return playerRow;
              })}
            </tbody>
          </table>
        </div>
      </CardBody>
    </Card>
  );
}
