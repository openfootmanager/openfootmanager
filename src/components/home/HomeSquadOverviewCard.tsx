import { AlertTriangle, Dumbbell } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { RatedTone } from "../../store/types";
import { Card, CardBody, CardHeader, ProgressBar } from "../ui";

interface HomeSquadOverviewCardProps {
  /** Null until the backend answers, and when no healthy first-team player can be averaged. */
  avgCondition: number | null;
  /** The backend's tone for the average; null leaves its bar neutral. */
  avgConditionTone: RatedTone | null;
  avgOvr: number;
  exhaustedCount: number;
  scheduleIcon: React.ReactNode;
  scheduleColorClass: string;
  scheduleLabel: string;
  focus: string;
  onNavigate?: (tab: string) => void;
}

export default function HomeSquadOverviewCard({
  avgCondition,
  avgConditionTone,
  avgOvr,
  exhaustedCount,
  scheduleIcon,
  scheduleColorClass,
  scheduleLabel,
  focus,
  onNavigate,
}: HomeSquadOverviewCardProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader
        action={
          <button
            type="button"
            onClick={() => onNavigate?.("Training")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("dashboard.training")}
          </button>
        }
      >
        {t("home.squadOverview")}
      </CardHeader>
      <CardBody>
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between">
            <span className="text-xs text-gray-500 dark:text-gray-400">
              {t("home.avgCondition")}
            </span>
            <span className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100">
              {avgCondition !== null ? `${avgCondition}%` : "—"}
            </span>
          </div>
          <ProgressBar
            value={avgCondition ?? 0}
            variant={avgConditionTone ? undefined : "muted"}
            tone={avgConditionTone ?? undefined}
            size="md"
            // No average, no progress bar: "0%" would be read out for a number that doesn't exist.
            label={avgCondition !== null ? t("home.avgCondition") : undefined}
          />

          <div className="flex items-center justify-between mt-1">
            <span className="text-xs text-gray-500 dark:text-gray-400">{t("home.avgOvr")}</span>
            <span className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100">
              {avgOvr}
            </span>
          </div>

          {exhaustedCount > 0 && (
            <div className="flex items-center gap-1.5 mt-1 text-amber-500 dark:text-amber-400">
              <AlertTriangle className="w-3.5 h-3.5" />
              <span className="text-xs font-heading">
                {t("home.exhaustedPlayers", { count: exhaustedCount })}
              </span>
            </div>
          )}

          <div className="mt-2 pt-2 border-t border-gray-100 dark:border-navy-700 flex items-center gap-2">
            <Dumbbell className="w-3.5 h-3.5 text-gray-400 dark:text-gray-500" />
            <span className="text-xs text-gray-500 dark:text-gray-400">
              {t("home.scheduleLabel")}
            </span>
            <span
              className={`text-xs font-heading font-bold flex items-center gap-1 ${scheduleColorClass}`}
            >
              {scheduleIcon} {scheduleLabel}
            </span>
            <span className="text-xs text-gray-400 dark:text-gray-500 ml-auto">
              {t(`common.trainingFocuses.${focus}`, focus)}
            </span>
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
