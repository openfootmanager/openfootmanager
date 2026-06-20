import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  getInjuryBadgeClassName,
  resolveInjuryName,
  type InjuryDisplayData,
} from "../../lib/injury";

interface InjuryBadgeProps {
  injury: InjuryDisplayData;
  showName?: boolean;
}

export function InjuryBadge({
  injury,
  showName = true,
}: InjuryBadgeProps) {
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
      className={`inline-flex max-w-44 items-center gap-1 rounded-md border px-2 py-0.5 font-heading text-xs font-bold uppercase tracking-wider ${getInjuryBadgeClassName(
        injury.days_remaining,
      )}`}
      title={title}
      aria-label={title}
    >
      <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
      {showName ? <span className="truncate">{injuryName}</span> : null}
      <span className="shrink-0 tabular-nums">{daysLabel}</span>
    </span>
  );
}
