import { CalendarClock } from "lucide-react";
import { useTranslation } from "react-i18next";

import { formatDateShort } from "../../lib/helpers";
import { Badge, Card, CardBody, CardHeader } from "../ui";

interface HomeSeasonStatusCardProps {
  phase: string;
  seasonStartLabel: string | null;
  daysUntilSeasonStart: number | null;
  transferWindowStatus: string;
  transferWindowVariant: "danger" | "success" | "neutral";
  transferWindowSummary: string;
  transferWindowOpensOn: string | null;
  transferWindowClosesOn: string | null;
  lang: string;
}

export default function HomeSeasonStatusCard({
  phase,
  seasonStartLabel,
  daysUntilSeasonStart,
  transferWindowStatus,
  transferWindowVariant,
  transferWindowSummary,
  transferWindowOpensOn,
  transferWindowClosesOn,
  lang,
}: HomeSeasonStatusCardProps) {
  const { t } = useTranslation();

  return (
    <Card accent="primary">
      <CardHeader>
        <div className="flex items-center gap-2">
          <CalendarClock className="w-4 h-4 text-primary-500" />
          {t("season.preseasonStatus")}
        </div>
      </CardHeader>
      <CardBody>
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div className="flex flex-col gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="accent" size="sm">
                {t(`season.phases.${phase}`)}
              </Badge>
              <Badge variant={transferWindowVariant} size="sm">
                {t(`season.transferWindowStatus.${transferWindowStatus}`)}
              </Badge>
            </div>
            <p className="text-sm text-gray-700 dark:text-gray-300">{t("season.preseasonFocus")}</p>
          </div>
          <div className="grid grid-cols-1 gap-3 md:grid-cols-2 md:min-w-[22rem]">
            <div className="rounded-xl bg-gray-50 px-4 py-3 dark:bg-navy-700/50">
              <p className="text-[10px] font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                {t("season.opener")}
              </p>
              <p className="mt-1 text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                {seasonStartLabel
                  ? t("season.startsOn", { date: seasonStartLabel })
                  : t("season.noOpener")}
              </p>
              {daysUntilSeasonStart !== null && (
                <p className="mt-1 text-xs text-primary-500 dark:text-primary-400">
                  {t("season.startsInDays", {
                    count: daysUntilSeasonStart,
                  })}
                </p>
              )}
            </div>
            <div className="rounded-xl bg-gray-50 px-4 py-3 dark:bg-navy-700/50">
              <p className="text-[10px] font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                {t("transfers.centre")}
              </p>
              <p className="mt-1 text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                {transferWindowSummary}
              </p>
              {(transferWindowOpensOn || transferWindowClosesOn) && (
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {transferWindowStatus === "Closed" && transferWindowOpensOn
                    ? t("season.windowOpensOn", {
                        date: formatDateShort(transferWindowOpensOn, lang),
                      })
                    : transferWindowClosesOn
                      ? t("season.windowClosesOn", {
                          date: formatDateShort(transferWindowClosesOn, lang),
                        })
                      : transferWindowSummary}
                </p>
              )}
            </div>
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
