import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  getInjurySeverity,
  resolveInjuryName,
  type InjuryDisplayData,
  type InjurySeverity,
} from "../../lib/injury";

/** Short injuries warn and long ones are danger; the stronger border marks the worse of each pair. */
const SEVERITY_CLASSES: Record<InjurySeverity, string> = {
  minor: "border-line-strong bg-warning-tint text-warning",
  moderate: "border-warning-solid bg-warning-tint text-warning",
  serious: "border-line-strong bg-danger-tint text-danger",
  major: "border-danger-solid bg-danger-tint text-danger",
};

interface InjuryBadgeProps {
  injury: InjuryDisplayData;
  showName?: boolean;
}

export function InjuryBadge({ injury, showName = true }: InjuryBadgeProps) {
  const { t } = useTranslation();
  const injuryName = resolveInjuryName(injury.name, t);
  const daysLabel = t("playerProfile.injuryDaysShort", {
    count: injury.days_remaining,
  });
  const title = `${injuryName} - ${t("playerProfile.daysRemaining", {
    count: injury.days_remaining,
  })}`;

  return (
    <span
      className={`inline-flex max-w-44 items-center gap-1 rounded-md border px-2 py-0.5 font-heading text-caption font-bold uppercase tracking-label ${
        SEVERITY_CLASSES[getInjurySeverity(injury.days_remaining)]
      }`}
      role="img"
      title={title}
      aria-label={title}
    >
      <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
      {showName ? <span className="truncate">{injuryName}</span> : null}
      <span className="shrink-0 tabular-nums">{daysLabel}</span>
    </span>
  );
}
