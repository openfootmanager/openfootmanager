import { AlertCircle } from "lucide-react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import { getFixtureDisplayLabel, getTeamName } from "../../lib/helpers";
import type { FixtureData, TeamData } from "../../store/gameStore";
import type { MatchModeType } from "../../hooks/useAdvanceTime";
import type { DashboardMatchModeMeta } from "./DashboardHeader";
import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardMatchConfirmModalProps {
  matchMode: MatchModeType;
  modeMeta: DashboardMatchModeMeta;
  onCancel: () => void;
  onConfirm: () => void;
  teams: TeamData[];
  todayMatchFixture: FixtureData | null;
}

export default function DashboardMatchConfirmModal({
  matchMode,
  modeMeta,
  onCancel,
  onConfirm,
  teams,
  todayMatchFixture,
}: DashboardMatchConfirmModalProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <DashboardModalFrame maxWidthClassName="max-w-md">
      <div className="mb-4 flex items-center gap-3">
        <div
          className={`flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br ${modeMeta.buttonColorClass} text-white`}
        >
          {modeMeta.icon}
        </div>
        <div>
          <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {t("continueMenu.matchDayTitle")}
          </h3>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            {modeMeta.label}
          </p>
        </div>
      </div>
      {todayMatchFixture && (
        <div className="mb-4 rounded-xl bg-gray-50 p-4 text-center dark:bg-navy-700">
          <p className="mb-2 text-xs font-heading uppercase tracking-widest text-gray-400">
            {getFixtureDisplayLabel(t, todayMatchFixture)}
          </p>
          <p className="text-lg font-heading font-bold text-gray-900 dark:text-white">
            {getTeamName(teams, todayMatchFixture.home_team_id)}{" "}
            <span className="mx-2 text-gray-400">{t("common.vs")}</span>{" "}
            {getTeamName(teams, todayMatchFixture.away_team_id)}
          </p>
        </div>
      )}
      <p className="mb-1 text-sm text-gray-500 dark:text-gray-400">
        {modeMeta.desc}
      </p>
      {matchMode === "delegate" && (
        <p className="mt-1 flex items-center gap-1 text-xs text-amber-500 dark:text-amber-400">
          <AlertCircle className="h-3.5 w-3.5" />
          {t("continueMenu.delegateWarning")}
        </p>
      )}
      <div className="mt-5 flex gap-3">
        <button
          onClick={onCancel}
          className="flex-1 rounded-lg bg-gray-100 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-gray-700 transition-colors hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-300 dark:hover:bg-navy-600"
        >
          {t("common.cancel")}
        </button>
        <button
          onClick={onConfirm}
          className={`flex flex-1 items-center justify-center gap-2 rounded-lg bg-gradient-to-r ${modeMeta.buttonColorClass} px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-white transition-all hover:brightness-110`}
        >
          {modeMeta.icon}
          {t("common.confirm")}
        </button>
      </div>
    </DashboardModalFrame>
  );
}
