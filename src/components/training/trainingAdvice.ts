type Translate = (key: string, params?: Record<string, string | number>) => string;

export interface TrainingAdviceParams {
  criticalCount: number;
  avgCondition: number;
  exhaustedCount: number;
  currentSchedule: string;
  currentFocus: string;
}

export interface TrainingAdvice {
  level: "ok" | "warn" | "critical";
  message: string;
}

export function getTrainingStaffAdvice(
  t: Translate,
  {
    criticalCount,
    avgCondition,
    exhaustedCount,
    currentSchedule,
    currentFocus,
  }: TrainingAdviceParams,
): TrainingAdvice | null {
  if (criticalCount >= 3) {
    const scheduleAdvice =
      currentSchedule === "Intense"
        ? t("training.staffAdvice.scheduleAdvice.criticalIntense")
        : currentSchedule === "Balanced"
          ? t("training.staffAdvice.scheduleAdvice.criticalBalanced")
          : t("training.staffAdvice.scheduleAdvice.criticalLight");

    return {
      level: "critical",
      message: t("training.staffAdvice.critical", {
        criticalCount,
        scheduleAdvice,
      }),
    };
  }

  if (avgCondition < 50 || exhaustedCount >= 4) {
    const scheduleAdvice =
      currentSchedule === "Intense"
        ? t("training.staffAdvice.scheduleAdvice.warnIntense")
        : currentSchedule === "Balanced"
          ? t("training.staffAdvice.scheduleAdvice.warnBalanced")
          : t("training.staffAdvice.scheduleAdvice.warnLight");

    return {
      level: "warn",
      message: t("training.staffAdvice.warn", {
        avgCondition,
        exhaustedCount,
        scheduleAdvice,
      }),
    };
  }

  if (avgCondition >= 80 && currentSchedule === "Light" && currentFocus !== "Recovery") {
    return {
      level: "ok",
      message: t("training.staffAdvice.ok"),
    };
  }

  return null;
}
