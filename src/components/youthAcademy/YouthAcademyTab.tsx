import { useEffect, useState } from "react";
import { GameStateData } from "../../store/gameStore";
import {
  Card,
  CardHeader,
  CardBody,
  Badge,
  ProgressBar,
  CountryFlag,
  Button,
} from "../ui";
import { calcAge, positionBadgeVariant } from "../../lib/helpers";
import { canDelegateToYouthAcademy, isYouthAcademyPlayer } from "../../lib/playerSquad";
import { TraitList } from "../TraitBadge";
import { useTranslation } from "react-i18next";
import { countryName } from "../../lib/countries";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import ContextMenu from "../ContextMenu";
import { buildPromoteToSeniorSquadMenuItem, buildViewProfileMenuItem } from "../playerActions/playerContextMenuItems";
import { setPlayerSquadRole } from "../../services/squadService";
import {
  cancelYouthScouting,
  reassignYouthScouting,
  startYouthScouting,
} from "../../services/scoutingService";
import { GraduationCap, ScanSearch, TrendingUp, Star, Users, Sparkles } from "lucide-react";
import type { DashboardNavigateContext } from "../dashboard/dashboardProfileNavigation";
import { calculateAvailableScouts } from "../scouting/ScoutingTab.helpers";
import ScoutingYouthRecruitmentCard from "../scouting/ScoutingYouthRecruitmentCard";

interface YouthAcademyTabProps {
  gameState: GameStateData;
  onSelectPlayer?: (id: string) => void;
  onGameUpdate?: (game: GameStateData) => void;
  onNavigate?: (tab: string, context?: DashboardNavigateContext) => void;
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
  onNavigate,
}: YouthAcademyTabProps) {
  const { t, i18n } = useTranslation();
  const [selectedYouthScoutId, setSelectedYouthScoutId] = useState("");
  const [youthRegion, setYouthRegion] = useState("Domestic");
  const [youthObjective, setYouthObjective] = useState("Balanced");
  const [youthTargetPosition, setYouthTargetPosition] = useState("");
  const [startingYouthSearch, setStartingYouthSearch] = useState(false);
  const [youthSearchError, setYouthSearchError] = useState<string | null>(null);
  const myTeam = gameState.teams.find(
    (tm) => tm.id === gameState.manager.team_id,
  );
  const scouts = gameState.staff.filter(
    (staffMember) =>
      staffMember.role === "Scout" && staffMember.team_id === gameState.manager.team_id,
  );
  const youthAssignments = gameState.youth_scouting_assignments || [];
  const availableScouts = calculateAvailableScouts(
    scouts,
    [...(gameState.scouting_assignments || []), ...youthAssignments],
  );

  useEffect(() => {
    if (
      selectedYouthScoutId &&
      availableScouts.some((scout) => scout.id === selectedYouthScoutId)
    ) {
      return;
    }

    setSelectedYouthScoutId(availableScouts[0]?.id ?? "");
  }, [availableScouts, selectedYouthScoutId]);

  const roster = myTeam
    ? gameState.players.filter((p) => p.team_id === myTeam.id)
    : [];
  const youthPlayers = roster
    .filter((player) => isYouthAcademyPlayer(player))
    .map((p) => ({
      ...p,
      age: calcAge(p.date_of_birth),
      ovr: p.ovr ?? 0,
      potential: p.potential ?? 1,
    }))
    .sort((a, b) => b.potential - a.potential);
  const eligibleSeniorPlayers = roster
    .filter((player) => canDelegateToYouthAcademy(player))
    .map((player) => ({
      ...player,
      age: calcAge(player.date_of_birth),
    }))
    .sort(
      (left, right) =>
        left.age - right.age || left.full_name.localeCompare(right.full_name),
    );

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

  const handleDelegatePlayer = async (playerId: string) => {
    try {
      const updated = await setPlayerSquadRole(playerId, "Youth");
      onGameUpdate?.(updated);
    } catch {
      return;
    }
  };

  const handleStartYouthScouting = async () => {
    if (!selectedYouthScoutId || !onGameUpdate) return;

    setStartingYouthSearch(true);
    setYouthSearchError(null);
    try {
      const updated = await startYouthScouting({
        scoutId: selectedYouthScoutId,
        region: youthRegion,
        objective: youthObjective,
        targetPosition: youthTargetPosition || null,
      });
      onGameUpdate(updated);
      setSelectedYouthScoutId("");
    } catch (err) {
      setYouthSearchError(String(err));
    } finally {
      setStartingYouthSearch(false);
    }
  };

  const handleCancelYouthScouting = async (assignmentId: string) => {
    if (!onGameUpdate) return;

    setYouthSearchError(null);
    try {
      const updated = await cancelYouthScouting(assignmentId);
      onGameUpdate(updated);
    } catch (err) {
      setYouthSearchError(String(err));
    }
  };

  const handleReassignYouthScouting = async (
    assignmentId: string,
    scoutId: string,
  ) => {
    if (!onGameUpdate) return;

    setYouthSearchError(null);
    try {
      const updated = await reassignYouthScouting(assignmentId, scoutId);
      onGameUpdate(updated);
    } catch (err) {
      setYouthSearchError(String(err));
    }
  };

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

      <Card accent="primary">
        <CardHeader
          action={
            onNavigate ? (
              <Button
                size="sm"
                variant="outline"
                icon={<ScanSearch />}
                onClick={() => onNavigate("Scouting")}
              >
                {t("youthAcademy.openScouting")}
              </Button>
            ) : undefined
          }
        >
          {t("youthAcademy.recoveryTitle")}
        </CardHeader>
        <CardBody className="flex flex-col gap-4">
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("youthAcademy.recoveryDescription")}
          </p>

          {eligibleSeniorPlayers.length > 0 ? (
            <div className="flex flex-col gap-3">
              <Badge variant="primary" size="sm" className="w-fit">
                {t("youthAcademy.eligibleSeniorPlayers", {
                  count: eligibleSeniorPlayers.length,
                })}
              </Badge>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {eligibleSeniorPlayers.slice(0, 4).map((player) => (
                  <div
                    key={player.id}
                    className="flex items-center justify-between gap-3 rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800/60 px-4 py-3"
                  >
                    <div className="min-w-0">
                      <button
                        onClick={() => onSelectPlayer?.(player.id)}
                        className="text-left font-heading font-bold text-sm text-gray-800 dark:text-gray-100 hover:text-primary-500 transition-colors truncate block"
                      >
                        {player.full_name}
                      </button>
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                        {translatePositionAbbreviation(
                          t,
                          player.natural_position || player.position,
                        )} · {t("youthAcademy.age")} {player.age}
                      </p>
                    </div>

                    <Button
                      size="sm"
                      onClick={() => {
                        void handleDelegatePlayer(player.id);
                      }}
                    >
                      {t("youthAcademy.delegateToYouthAcademy")}
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {t("youthAcademy.noEligibleSeniorPlayers")}
            </p>
          )}
        </CardBody>
      </Card>

      {scouts.length > 0 ? (
        <ScoutingYouthRecruitmentCard
          title={t("youthAcademy.recruitmentWorkflowTitle")}
          hint={t("youthAcademy.recruitmentWorkflowHint")}
          youthAssignments={youthAssignments}
          scouts={scouts}
          availableScouts={availableScouts}
          isStarting={startingYouthSearch}
          selectedScoutId={selectedYouthScoutId}
          region={youthRegion}
          objective={youthObjective}
          targetPosition={youthTargetPosition}
          errorMessage={youthSearchError}
          onScoutChange={setSelectedYouthScoutId}
          onRegionChange={setYouthRegion}
          onObjectiveChange={setYouthObjective}
          onTargetPositionChange={setYouthTargetPosition}
          onStartSearch={() => {
            void handleStartYouthScouting();
          }}
          onCancelSearch={(assignmentId) => {
            void handleCancelYouthScouting(assignmentId);
          }}
          onReassignSearch={(assignmentId, scoutId) => {
            void handleReassignYouthScouting(assignmentId, scoutId);
          }}
        />
      ) : null}

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
