import { Calendar, RotateCcw, Trash2, TimerOff, UserPlus } from "lucide-react";
import { getContractYearsRemaining } from "../../lib/helpers";
import { Button, Card, CardBody, CardHeader } from "../ui";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileContractCardProps {
  contractEnd: string | null;
  currentDate: string;
  isOwnClub: boolean;
  isFreeAgent?: boolean;
  hasLetExpireIntent: boolean;
  actionSubmitting: boolean;
  onOpenRenewal: () => void;
  onMarkLetExpire: () => void;
  onClearLetExpire: () => void;
  onOpenTermination: () => void;
  onOpenFreeAgentContract?: () => void;
  t: TranslateFn;
}

export default function PlayerProfileContractCard({
  contractEnd,
  currentDate,
  isOwnClub,
  isFreeAgent,
  hasLetExpireIntent,
  actionSubmitting,
  onOpenRenewal,
  onMarkLetExpire,
  onClearLetExpire,
  onOpenTermination,
  onOpenFreeAgentContract,
  t,
}: PlayerProfileContractCardProps) {
  return (
    <Card>
      <CardHeader>{t("playerProfile.contractInfo")}</CardHeader>
      <CardBody>
        <div className="flex flex-col gap-3">
          <InfoRow
            icon={<Calendar className="w-4 h-4" />}
            label={t("playerProfile.yearsRemaining")}
            value={getContractYearsRemaining(contractEnd, currentDate)}
          />
        </div>
        {isOwnClub ? (
          <div className="flex flex-wrap gap-2 pt-3">
            {hasLetExpireIntent ? (
              <Button
                size="sm"
                variant="outline"
                icon={<RotateCcw />}
                disabled={actionSubmitting}
                onClick={onClearLetExpire}
              >
                {t("playerProfile.reopenContractTalks")}
              </Button>
            ) : (
              <>
                <Button size="sm" variant="outline" onClick={onOpenRenewal}>
                  {t("common.renewContract")}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  icon={<TimerOff />}
                  disabled={actionSubmitting}
                  onClick={onMarkLetExpire}
                >
                  {t("playerProfile.letContractExpire")}
                </Button>
              </>
            )}
            <Button
              size="sm"
              variant="outline"
              icon={<Trash2 />}
              className="text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
              disabled={actionSubmitting || contractEnd === null}
              onClick={onOpenTermination}
            >
              {t("playerProfile.terminateContract")}
            </Button>
          </div>
        ) : isFreeAgent && onOpenFreeAgentContract ? (
          <div className="flex flex-wrap gap-2 pt-3">
            <Button
              size="sm"
              variant="outline"
              icon={<UserPlus />}
              onClick={onOpenFreeAgentContract}
            >
              {t("transfers.offerContract")}
            </Button>
          </div>
        ) : null}
      </CardBody>
    </Card>
  );
}

function InfoRow({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-3 py-2 border-b border-gray-100 dark:border-navy-600 last:border-0">
      <div className="text-gray-400 dark:text-gray-500">{icon}</div>
      <span className="text-sm text-gray-500 dark:text-gray-400 flex-1">{label}</span>
      <span className="text-sm font-semibold text-gray-800 dark:text-gray-200">{value}</span>
    </div>
  );
}
