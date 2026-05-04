import { GameStateData, PlayerData } from "../../store/gameStore";
import {
  Card,
  CardHeader,
  CardBody,
  Badge,
  ProgressBar,
  CountryFlag,
} from "../ui";
import { calcOvr, calcAge, positionBadgeVariant } from "../../lib/helpers";
import { isYouthAcademyPlayer } from "../../lib/playerSquad";
import { TraitList } from "../TraitBadge";
import { useTranslation } from "react-i18next";
import { countryName } from "../../lib/countries";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import ContextMenu from "../ContextMenu";
import { buildPromoteToSeniorSquadMenuItem, buildViewProfileMenuItem } from "../playerActions/playerContextMenuItems";
import { setPlayerSquadRole } from "../../services/squadService";
import { GraduationCap, TrendingUp, Star, Users, Sparkles } from "lucide-react";

interface YouthAcademyTabProps {
  gameState: GameStateData;
  onSelectPlayer?: (id: string) => void;
  onGameUpdate?: (game: GameStateData) => void;
}

// Estimate potential: younger players with good attributes have higher ceiling
function estimatePotential(player: PlayerData): number {
  const ovr = calcOvr(player, player.natural_position || player.position);
  const age = calcAge(player.date_of_birth);
  // Young players get a bonus: the younger they are with decent OVR, the higher the ceiling
  const ageFactor = Math.max(0, (23 - age) * 2.5); // +2.5 per year under 23
  const potential = Math.min(99, Math.round(ovr + ageFactor));
  return potential;
}

function getPotentialLabel(
  potential: number,
  t: (key: string) => string,
): { label: string; color: string } {
  if (potential >= 85)
    return { label: t("youthAcademy.potWorldClass"), color: "text-accent-400" };
  if (potential >= 75)
    return { label: t("youthAcademy.potExcellent"), color: "text-green-400" };
  if (potential >= 65)
    return { label: t("youthAcademy.potPromising"), color: "text-primary-400" };
  if (potential >= 55)
    return { label: t("youthAcademy.potDecent"), color: "text-gray-400" };
  return { label: t("youthAcademy.potLimited"), color: "text-gray-500" };
}

export default function YouthAcademyTab({
  gameState,
  onSelectPlayer,
  onGameUpdate,
}: YouthAcademyTabProps) {
  const { t, i18n } = useTranslation();
  const myTeam = gameState.teams.find(
    (tm) => tm.id === gameState.manager.team_id,
  );

  const roster = myTeam
    ? gameState.players.filter((p) => p.team_id === myTeam.id)
    : [];
  const youthPlayers = roster
    .filter((player) => isYouthAcademyPlayer(player))
    .map((p) => ({
      ...p,
      age: calcAge(p.date_of_birth),
      ovr: calcOvr(p, p.natural_position || p.position),
      potential: estimatePotential(p),
    }))
    .sort((a, b) => b.potential - a.potential);

  const avgOvr =
    youthPlayers.length > 0
      ? Math.round(
        youthPlayers.reduce((s, p) => s + p.ovr, 0) / youthPlayers.length,
      )
      : 0;
  const avgPotential =
    youthPlayers.length > 0
      ? Math.round(
        youthPlayers.reduce((s, p) => s + p.potential, 0) /
        youthPlayers.length,
      )
      : 0;
  const highPotential = youthPlayers.filter((p) => p.potential >= 75).length;

  // Youth development staff
  const youthCoach = gameState.staff.filter(
    (s) => s.team_id === myTeam?.id && s.specialization === "Youth",
  );

  return (
    <div className="max-w-5xl mx-auto flex flex-col gap-5">
      {/* Header */}
      <div className="flex items-center gap-3">
        <GraduationCap className="w-5 h-5 text-primary-500" />
        <h2 className="text-lg font-heading font-bold text-gray-800 dark:text-gray-100 uppercase tracking-wider">
          {t("youthAcademy.title")}
        </h2>
        <Badge variant="neutral" size="sm">
          {t("youthAcademy.playersUnder21", { count: youthPlayers.length })}
        </Badge>
      </div>

      {/* Overview Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card>
          <CardBody>
            <div className="text-center">
              <Users className="w-5 h-5 text-gray-400 dark:text-gray-500 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">
                {youthPlayers.length}
              </p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {t("youthAcademy.youthPlayers")}
              </p>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="text-center">
              <Star className="w-5 h-5 text-accent-400 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">
                {avgOvr}
              </p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {t("youthAcademy.avgOvr")}
              </p>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="text-center">
              <TrendingUp className="w-5 h-5 text-green-500 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">
                {avgPotential}
              </p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {t("youthAcademy.avgPotential")}
              </p>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="text-center">
              <Sparkles className="w-5 h-5 text-accent-400 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-accent-500">
                {highPotential}
              </p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {t("youthAcademy.highPotential")}
              </p>
            </div>
          </CardBody>
        </Card>
      </div>

      {/* Youth Staff */}
      {youthCoach.length > 0 && (
        <Card>
          <CardBody>
            <div className="flex items-center gap-2 text-xs">
              <GraduationCap className="w-3.5 h-3.5 text-primary-500" />
              <span className="text-gray-500 dark:text-gray-400">
                {t("youthAcademy.youthCoach")}
              </span>
              {youthCoach.map((s) => (
                <Badge key={s.id} variant="primary" size="sm">
                  {s.first_name} {s.last_name} ({s.attributes.coaching})
                </Badge>
              ))}
            </div>
          </CardBody>
        </Card>
      )}

      {/* Youth Players Table */}
      <Card>
        <CardHeader>{t("youthAcademy.youthProspects")}</CardHeader>
        <CardBody className="p-0">
          {youthPlayers.length === 0 ? (
            <div className="flex flex-col items-center gap-3 py-12">
              <GraduationCap className="w-10 h-10 text-gray-300 dark:text-navy-600" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {t("youthAcademy.noYouthPlayers")}
              </p>
            </div>
          ) : (
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("youthAcademy.player")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("youthAcademy.pos")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                    {t("youthAcademy.age")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                    {t("youthAcademy.ovr")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                    {t("youthAcademy.potential")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("youthAcademy.growth")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("youthAcademy.traits")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                    {t("youthAcademy.condition")}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {youthPlayers.map((player) => {
                  const potLabel = getPotentialLabel(player.potential, t);
                  const growthRoom = player.potential - player.ovr;
                  const contextItems = [
                    buildViewProfileMenuItem(t, () => onSelectPlayer?.(player.id)),
                    buildPromoteToSeniorSquadMenuItem(t, async () => {
                      try {
                        const updated = await setPlayerSquadRole(player.id, "Senior");
                        onGameUpdate?.(updated);
                      } catch {
                        return;
                      }
                    }),
                  ];

                  return (
                    <ContextMenu items={contextItems} key={player.id}>
                      <tr
                        onClick={() => onSelectPlayer?.(player.id)}
                        className="hover:bg-gray-50 dark:hover:bg-navy-700/50 cursor-pointer transition-colors"
                      >
                        <td className="py-2.5 px-4">
                          <div>
                            <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                              {player.full_name}
                            </p>
                            <div className="text-[10px] text-gray-400 dark:text-gray-500 flex items-center gap-1 mt-0.5">
                              <CountryFlag
                                code={player.nationality}
                                locale={i18n.language}
                                className="text-xs leading-none"
                              />
                              <span>
                                {countryName(player.nationality, i18n.language)}
                              </span>
                            </div>
                          </div>
                        </td>
                        <td className="py-2.5 px-4">
                          <Badge
                            variant={positionBadgeVariant(
                              player.natural_position || player.position,
                            )}
                            size="sm"
                          >
                            {translatePositionAbbreviation(
                              t,
                              player.natural_position || player.position,
                            )}
                          </Badge>
                        </td>
                        <td className="py-2.5 px-4 text-center">
                          <span className="text-sm font-heading font-bold text-gray-700 dark:text-gray-300 tabular-nums">
                            {player.age}
                          </span>
                        </td>
                        <td className="py-2.5 px-4 text-center">
                          <span className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100 tabular-nums">
                            {player.ovr}
                          </span>
                        </td>
                        <td className="py-2.5 px-4 text-center">
                          <span
                            className={`text-sm font-heading font-bold tabular-nums ${potLabel.color}`}
                          >
                            {player.potential}
                          </span>
                          <p
                            className={`text-[9px] font-heading uppercase tracking-wider ${potLabel.color}`}
                          >
                            {potLabel.label}
                          </p>
                        </td>
                        <td className="py-2.5 px-4">
                          <div className="flex items-center gap-2">
                            <ProgressBar
                              value={Math.min(
                                100,
                                (player.ovr / player.potential) * 100,
                              )}
                              variant={
                                growthRoom > 15
                                  ? "accent"
                                  : growthRoom > 5
                                    ? "primary"
                                    : "auto"
                              }
                              size="sm"
                            />
                            <span className="text-[10px] font-heading font-bold text-gray-500 tabular-nums w-6">
                              +{growthRoom}
                            </span>
                          </div>
                        </td>
                        <td className="py-2.5 px-4">
                          <TraitList traits={player.traits || []} max={2} />
                        </td>
                        <td className="py-2.5 px-4 text-center">
                          <span
                            className={`text-xs font-heading font-bold tabular-nums ${player.condition >= 70
                                ? "text-green-500"
                                : player.condition >= 40
                                  ? "text-yellow-500"
                                  : "text-red-500"
                              }`}
                          >
                            {player.condition}%
                          </span>
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
    </div>
  );
}
