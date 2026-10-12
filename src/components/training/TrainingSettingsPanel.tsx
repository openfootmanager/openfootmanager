import { Gauge } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { TrainingFocusAttributesData } from "../../store/types";
import { Card, CardBody, CardHeader } from "../ui";

interface TrainingSettingsPanelProps {
  currentFocus: string;
  currentIntensity: string;
  currentSchedule: string;
  isSaving: boolean;
  todayWeekday: number;
  isTodayTraining: boolean;
  onSetTraining: (focus: string, intensity: string) => void;
  onSetSchedule: (schedule: string) => void;
  scheduleIds: readonly string[];
  scheduleIcons: Record<string, React.ReactNode>;
  scheduleColors: Record<string, string>;
  dayKeys: readonly string[];
  trainingFocusIds: readonly string[];
  trainingFocusIcons: Record<string, React.ReactNode>;
  focusAttributes: Record<string, TrainingFocusAttributesData>;
  intensityIds: readonly string[];
  intensityColors: Record<string, string>;
}

function goalkeeperOnlyAttributes(entry: TrainingFocusAttributesData | undefined): string[] {
  if (!entry) return [];
  const sameAsOutfield =
    entry.goalkeeper.length === entry.outfield.length &&
    entry.goalkeeper.every((attribute, index) => attribute === entry.outfield[index]);
  return sameAsOutfield ? [] : entry.goalkeeper;
}

function FocusAttributeChips({
  attributes,
  label,
}: {
  attributes: readonly string[];
  label: string | null;
}) {
  const { t } = useTranslation();
  if (attributes.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1 mt-2">
      {label && (
        <span className="text-[10px] font-heading font-bold uppercase tracking-wider text-gray-600 dark:text-gray-300">
          {label}:
        </span>
      )}
      {attributes.map((attribute) => (
        <span
          key={attribute}
          className="text-[10px] bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 px-1.5 py-0.5 rounded font-heading uppercase tracking-wider"
        >
          {t(`common.attributes.${attribute}`)}
        </span>
      ))}
    </div>
  );
}

export default function TrainingSettingsPanel({
  currentFocus,
  currentIntensity,
  currentSchedule,
  isSaving,
  todayWeekday,
  isTodayTraining,
  onSetTraining,
  onSetSchedule,
  scheduleIds,
  scheduleIcons,
  scheduleColors,
  dayKeys,
  trainingFocusIds,
  trainingFocusIcons,
  focusAttributes,
  intensityIds,
  intensityColors,
}: TrainingSettingsPanelProps) {
  const { t } = useTranslation();
  const activeFocusAttrs = focusAttributes[currentFocus]?.outfield ?? [];
  const activeGoalkeeperAttrs = goalkeeperOnlyAttributes(focusAttributes[currentFocus]);
  const attributeNames = (attributes: readonly string[]) =>
    attributes.map((attribute) => t(`common.attributes.${attribute}`)).join(", ");

  return (
    <>
      <Card accent="accent">
        <CardHeader>{t("training.weeklySchedule")}</CardHeader>
        <CardBody>
          <div className="flex gap-3 mb-4">
            {scheduleIds.map((scheduleId) => (
              <button
                type="button"
                key={scheduleId}
                disabled={isSaving}
                onClick={() => onSetSchedule(scheduleId)}
                className={`flex-1 p-3 rounded-xl text-left transition-all border-2 ${
                  currentSchedule === scheduleId
                    ? "border-primary-500 bg-primary-50 dark:bg-primary-500/10 shadow-md shadow-primary-500/10"
                    : "border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-500"
                } ${isSaving ? "opacity-60 pointer-events-none" : ""}`}
              >
                <div className={`mb-1.5 ${scheduleColors[scheduleId]}`}>
                  {scheduleIcons[scheduleId]}
                </div>
                <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-800 dark:text-gray-200">
                  {t(`training.schedules.${scheduleId}.label`)}
                </p>
                <p className="text-[11px] text-gray-500 dark:text-gray-400 mt-1">
                  {t(`training.schedules.${scheduleId}.desc`)}
                </p>
              </button>
            ))}
          </div>

          <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">
            {t(`training.schedules.${currentSchedule}.detail`)}{" "}
            <span
              dangerouslySetInnerHTML={{
                __html: t("training.todayIs", {
                  day: t(`training.days.${dayKeys[todayWeekday]}`),
                  type: isTodayTraining ? t("training.aTrainingDay") : t("training.aRestDay"),
                }),
              }}
            />
          </p>
        </CardBody>
      </Card>

      <Card accent="primary">
        <CardHeader>{t("training.trainingFocus")}</CardHeader>
        <CardBody>
          <div className="grid grid-cols-3 gap-3">
            {trainingFocusIds.map((focusId) => (
              <button
                type="button"
                key={focusId}
                disabled={isSaving}
                onClick={() => onSetTraining(focusId, currentIntensity)}
                className={`p-4 rounded-xl text-left transition-all border-2 ${
                  currentFocus === focusId
                    ? "border-primary-500 bg-primary-50 dark:bg-primary-500/10 shadow-md shadow-primary-500/10"
                    : "border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-500"
                } ${isSaving ? "opacity-60 pointer-events-none" : ""}`}
              >
                <div className="mb-2 text-gray-600 dark:text-gray-300">
                  {trainingFocusIcons[focusId]}
                </div>
                <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-800 dark:text-gray-200">
                  {t(`training.focuses.${focusId}.label`)}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  {t(`training.focuses.${focusId}.desc`)}
                </p>
                <FocusAttributeChips
                  attributes={focusAttributes[focusId]?.outfield ?? []}
                  label={null}
                />
                <FocusAttributeChips
                  attributes={goalkeeperOnlyAttributes(focusAttributes[focusId])}
                  label={t("training.goalkeepersLabel")}
                />
              </button>
            ))}
          </div>

          <div className="mt-5 pt-4 border-t border-gray-100 dark:border-navy-700">
            <div className="flex items-center gap-2 mb-3">
              <Gauge className="w-4 h-4 text-gray-500 dark:text-gray-400" />
              <span className="text-xs font-heading font-bold uppercase tracking-widest text-gray-600 dark:text-gray-400">
                {t("training.intensity")}
              </span>
            </div>
            <div className="flex gap-3">
              {intensityIds.map((intensityId) => (
                <button
                  type="button"
                  key={intensityId}
                  disabled={isSaving}
                  onClick={() => onSetTraining(currentFocus, intensityId)}
                  className={`flex-1 p-3 rounded-lg text-left transition-all border-2 ${
                    currentIntensity === intensityId
                      ? "border-primary-500 bg-primary-50 dark:bg-primary-500/10"
                      : "border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-500"
                  } ${isSaving ? "opacity-60 pointer-events-none" : ""}`}
                >
                  <p
                    className={`font-heading font-bold text-sm uppercase tracking-wider ${intensityColors[intensityId]}`}
                  >
                    {t(`training.intensities.${intensityId}.label`)}
                  </p>
                  <p className="text-[10px] text-gray-500 dark:text-gray-400 mt-0.5">
                    {t(`training.intensities.${intensityId}.desc`)}
                  </p>
                </button>
              ))}
            </div>
          </div>

          <p className="text-xs text-gray-400 dark:text-gray-500 mt-4">
            {t("training.trainingAppliedNote")}
            {activeFocusAttrs.length > 0 && (
              <>
                {" "}
                <span
                  dangerouslySetInnerHTML={{
                    __html: t("training.currentlyTraining", {
                      attrs: attributeNames(activeFocusAttrs),
                      intensity: t(`training.intensities.${currentIntensity}.label`),
                    }),
                  }}
                />
              </>
            )}
            {activeGoalkeeperAttrs.length > 0 && (
              <>
                {" "}
                {t("training.goalkeepersLabel")}: {attributeNames(activeGoalkeeperAttrs)}.
              </>
            )}
            {currentFocus === "Recovery" && <> {t("training.recoveryNote")}</>}
          </p>
        </CardBody>
      </Card>
    </>
  );
}
