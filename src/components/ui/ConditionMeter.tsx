import { useTranslation } from "react-i18next";

import type { ConditionStatus } from "../../store/types";
import { ProgressBar } from "./ProgressBar";
import { TONE_TEXT } from "./ratedTone";

interface ConditionMeterProps {
  /** The player's condition, 0 to 100. */
  value: number;
  /** The backend's band for it; null until the squad's projection arrives. */
  status: ConditionStatus | null | undefined;
  className?: string;
}

/**
 * A player's condition as table C draws it: the backend's band word in its tone, a bar in the same
 * tone and the percentage. The band is never worked out here: without the backend's, the bar is
 * neutral and there is no word.
 */
export function ConditionMeter({ value, status, className = "" }: ConditionMeterProps) {
  const { t } = useTranslation();
  return (
    <div className={`flex items-center gap-2 ${className}`}>
      {status ? (
        <span
          className={`w-16 shrink-0 truncate text-caption font-semibold ${TONE_TEXT[status.tone]}`}
        >
          {t(status.label_key)}
        </span>
      ) : null}
      <ProgressBar
        value={value}
        tone={status?.tone}
        variant={status ? undefined : "muted"}
        size="sm"
        showLabel
        label={t("common.condition")}
        className="w-24"
      />
    </div>
  );
}
