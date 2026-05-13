import { useState, type ReactNode } from "react";
import {
  AlertTriangle,
  BedDouble,
  Brain,
  Crosshair,
  Feather,
  Flame,
  HeartPulse,
  Info,
  Scale,
  Shield,
  Zap,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { GameStateData } from "../../store/gameStore";
import { isSeniorSquadPlayer } from "../../lib/playerSquad";
import { setTraining, setTrainingSchedule } from "../../services/trainingService";
import { Card, CardBody, CardHeader, ProgressBar } from "../ui";
import TrainingGroupsCard from "./TrainingGroupsCard";
import TrainingSettingsPanel from "./TrainingSettingsPanel";
import { getTrainingStaffAdvice } from "./trainingAdvice";

interface TrainingTabProps {
  gameState: GameStateData;
  onGameUpdate?: (state: GameStateData) => void;
}

const TRAINING_FOCUS_IDS = [
  "Physical",
  "Technical",
  "Tactical",
  "Defending",
  "Attacking",
  "Recovery",
] as const;

const TRAINING_FOCUS_ICONS: Record<string, ReactNode> = {
  Physical: <HeartPulse className="w-6 h-6" />,
  Technical: <Crosshair className="w-6 h-6" />,
  Tactical: <Brain className="w-6 h-6" />,
  Defending: <Shield className="w-6 h-6" />,
  Attacking: <Zap className="w-6 h-6" />,
  Recovery: <BedDouble className="w-6 h-6" />,
};

const TRAINING_FOCUS_ATTRS: Record<string, string[]> = {
  Physical: ["pace", "stamina", "strength", "agility"],
  Technical: ["passing", "shooting", "dribbling"],
  Tactical: ["positioning", "vision", "decisions", "composure"],
  Defending: ["tackling", "defending", "strength", "positioning"],
  Attacking: ["shooting", "dribbling", "pace"],
  Recovery: [],
};

const INTENSITY_IDS = ["Low", "Medium", "High"] as const;

const INTENSITY_COLORS: Record<string, string> = {
  Low: "text-blue-500",
  Medium: "text-accent-500",
  High: "text-red-500",
};

const SCHEDULE_IDS = ["Intense", "Balanced", "Light"] as const;

const SCHEDULE_ICONS: Record<string, ReactNode> = {
  Intense: <Flame className="w-5 h-5" />,
  Balanced: <Scale className="w-5 h-5" />,
  Light: <Feather className="w-5 h-5" />,
};

const SCHEDULE_COLORS: Record<string, string> = {
  Intense: "text-red-500",
  Balanced: "text-primary-500",
  Light: "text-blue-500",
};

const SCHEDULE_TRAINING_DAYS: Record<string, number[]> = {
  Intense: [0, 1, 2, 3, 4, 5],
  Balanced: [0, 1, 3, 4],
  Light: [1, 3],
};

const DAY_KEYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const;

function getWeekdayFromDate(dateStr: string): number {
  const date = new Date(dateStr);
  return (date.getUTCDay() + 6) % 7;
}

export default function TrainingTab({
  gameState,
  onGameUpdate,
}: TrainingTabProps) {
  const { t } = useTranslation();
  const myTeam = gameState.teams.find(
    (team) => team.id === gameState.manager.team_id,
  );

  if (!myTeam) {
    return (
      <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>
    );
  }

  const currentFocus = myTeam.training_focus || "Physical";
  const currentIntensity = myTeam.training_intensity || "Medium";
  const currentSchedule = myTeam.training_schedule || "Balanced";
  const [isSaving, setIsSaving] = useState(false);

  const roster = gameState.players.filter(
    (player) => player.team_id === myTeam.id && isSeniorSquadPlayer(player),
  );
  const avgCondition =
    roster.length > 0
      ? Math.round(
        roster.reduce((sum, player) => sum + player.condition, 0) / roster.length,
      )
      : 0;
  const avgMorale =
    roster.length > 0
      ? Math.round(
        roster.reduce((sum, player) => sum + player.morale, 0) / roster.length,
      )
      : 0;
  const exhaustedCount = roster.filter((player) => player.condition < 40).length;
  const criticalCount = roster.filter((player) => player.condition < 25).length;

  const todayWeekday = getWeekdayFromDate(gameState.clock.current_date);
  const trainingDays =
    SCHEDULE_TRAINING_DAYS[currentSchedule] || SCHEDULE_TRAINING_DAYS.Balanced;
  const isTodayTraining = trainingDays.includes(todayWeekday);

  const handleSetTraining = async (focus: string, intensity: string) => {
    setIsSaving(true);
    try {
      const updated = await setTraining(focus, intensity);
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to set training:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const handleSetSchedule = async (schedule: string) => {
    setIsSaving(true);
    try {
      const updated = await setTrainingSchedule(schedule);
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to set schedule:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const activeFocusAttrs = TRAINING_FOCUS_ATTRS[currentFocus] || [];
  const staffAdvice = getTrainingStaffAdvice(t, {
    criticalCount,
    avgCondition,
    exhaustedCount,
    currentSchedule,
    currentFocus,
  });

  return (
    <div className="max-w-6xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-5">
      <div className="lg:col-span-2 flex flex-col gap-5">
        {staffAdvice ? (
          <div
            className={`flex items-start gap-3 p-4 rounded-xl border-2 ${staffAdvice.level === "critical"
                ? "bg-red-50 dark:bg-red-500/10 border-red-300 dark:border-red-500/40"
                : staffAdvice.level === "warn"
                  ? "bg-amber-50 dark:bg-amber-500/10 border-amber-300 dark:border-amber-500/40"
                  : "bg-blue-50 dark:bg-blue-500/10 border-blue-300 dark:border-blue-500/40"
              }`}
          >
            {staffAdvice.level === "critical" ? (
              <AlertTriangle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
            ) : staffAdvice.level === "warn" ? (
              <AlertTriangle className="w-5 h-5 text-amber-500 flex-shrink-0 mt-0.5" />
            ) : (
              <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
            )}
            <div>
              <p
                className={`text-xs font-heading font-bold uppercase tracking-wider mb-0.5 ${staffAdvice.level === "critical"
                    ? "text-red-600 dark:text-red-400"
                    : staffAdvice.level === "warn"
                      ? "text-amber-600 dark:text-amber-400"
                      : "text-blue-600 dark:text-blue-400"
                  }`}
              >
                {staffAdvice.level === "critical"
                  ? t("training.staffAlert")
                  : staffAdvice.level === "warn"
                    ? t("training.staffWarning")
                    : t("training.staffSuggestion")}
              </p>
              <p className="text-sm text-gray-700 dark:text-gray-300">
                {staffAdvice.message}
              </p>
            </div>
          </div>
        ) : null}

        <TrainingSettingsPanel
          currentFocus={currentFocus}
          currentIntensity={currentIntensity}
          currentSchedule={currentSchedule}
          isSaving={isSaving}
          todayWeekday={todayWeekday}
          isTodayTraining={isTodayTraining}
          activeFocusAttrs={activeFocusAttrs}
          onSetTraining={handleSetTraining}
          onSetSchedule={handleSetSchedule}
          scheduleIds={SCHEDULE_IDS}
          scheduleIcons={SCHEDULE_ICONS}
          scheduleColors={SCHEDULE_COLORS}
          dayKeys={DAY_KEYS}
          trainingFocusIds={TRAINING_FOCUS_IDS}
          trainingFocusIcons={TRAINING_FOCUS_ICONS}
          trainingFocusAttrs={TRAINING_FOCUS_ATTRS}
          intensityIds={INTENSITY_IDS}
          intensityColors={INTENSITY_COLORS}
        />

        <TrainingGroupsCard
          gameState={gameState}
          onGameUpdate={onGameUpdate}
          roster={roster}
          isSaving={isSaving}
          setIsSaving={setIsSaving}
          trainingFocusIds={TRAINING_FOCUS_IDS}
          trainingFocusIcons={TRAINING_FOCUS_ICONS}
        />
      </div>

      <div className="flex flex-col gap-5">
        <Card accent="accent">
          <CardHeader>{t("training.squadFitness")}</CardHeader>
          <CardBody>
            <div className="flex flex-col gap-3">
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-gray-600 dark:text-gray-400">
                    {t("training.avgCondition")}
                  </span>
                  <span className="font-heading font-bold text-gray-800 dark:text-gray-100">
                    {avgCondition}%
                  </span>
                </div>
                <ProgressBar value={avgCondition} variant="auto" size="md" />
              </div>
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-gray-600 dark:text-gray-400">
                    {t("training.avgMorale")}
                  </span>
                  <span className="font-heading font-bold text-gray-800 dark:text-gray-100">
                    {avgMorale}%
                  </span>
                </div>
                <ProgressBar value={avgMorale} variant="auto" size="md" />
              </div>
              {exhaustedCount > 0 || criticalCount > 0 ? (
                <div className="mt-1 pt-2 border-t border-gray-100 dark:border-navy-700">
                  {criticalCount > 0 ? (
                    <p className="text-xs text-red-500 dark:text-red-400 flex items-center gap-1">
                      <AlertTriangle className="w-3 h-3" />{" "}
                      {t("training.criticalCondition", { count: criticalCount })}
                    </p>
                  ) : null}
                  {exhaustedCount > 0 ? (
                    <p className="text-xs text-amber-500 dark:text-amber-400 flex items-center gap-1 mt-0.5">
                      <AlertTriangle className="w-3 h-3" />{" "}
                      {t("training.exhaustedPlayers", { count: exhaustedCount })}
                    </p>
                  ) : null}
                </div>
              ) : null}
            </div>
          </CardBody>
        </Card>

        <Card>
          <CardHeader>{t("training.playerFitness")}</CardHeader>
          <CardBody className="p-0 max-h-64 overflow-y-auto">
            <div className="divide-y divide-gray-100 dark:divide-navy-600">
              {[...roster]
                .sort((left, right) => left.condition - right.condition)
                .map((player) => (
                  <div key={player.id} className="flex items-center px-4 py-2 gap-3">
                    <span
                      className={`text-sm font-medium flex-1 truncate ${player.condition < 25
                          ? "text-red-600 dark:text-red-400"
                          : player.condition < 40
                            ? "text-amber-600 dark:text-amber-400"
                            : "text-gray-800 dark:text-gray-200"
                        }`}
                    >
                      {player.match_name}
                    </span>
                    <ProgressBar
                      value={player.condition}
                      variant="auto"
                      size="sm"
                      showLabel
                      className="w-24"
                    />
                  </div>
                ))}
            </div>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}
