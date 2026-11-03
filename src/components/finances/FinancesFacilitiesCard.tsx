import { useTranslation } from "react-i18next";
import { Card, CardHeader, CardBody, Button } from "../ui";
import { formatExactMoney } from "../../lib/helpers";
import { resolveBackendError } from "../../utils/backendI18n";
import type { FacilityUpgradePreviewData } from "../../services/financeService";
import {
  type FacilityId,
  type FacilityUpgradeErrorState,
  FACILITY_DEFINITIONS,
} from "./FinancesTab.helpers";

interface FinancesFacilitiesCardProps {
  facilities: Record<"training" | "medical" | "scouting", number>;
  /** The backend's verdict per facility; the card decides nothing itself. */
  facilityUpgrades: FacilityUpgradePreviewData[];
  /** False while the verdict describes an older club state: every upgrade stays disabled. */
  previewsCurrent: boolean;
  facilityUpgradeError: FacilityUpgradeErrorState | null;
  actionLoading: string | null;
  onUpgrade: (facility: FacilityId) => void;
}

export default function FinancesFacilitiesCard({
  facilities,
  facilityUpgrades,
  previewsCurrent,
  facilityUpgradeError,
  actionLoading,
  onUpgrade,
}: FinancesFacilitiesCardProps) {
  const { t } = useTranslation();

  return (
    <Card className="lg:col-span-3">
      <CardHeader>{t("finances.facilities")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {FACILITY_DEFINITIONS.map((facility) => {
            const level = facilities[facility.levelKey];
            const upgrade = facilityUpgrades.find((entry) => entry.facility === facility.id);
            const financeBlockReason = previewsCurrent ? upgrade?.blockReason : null;
            const canAffordUpgrade = upgrade?.canAfford ?? true;
            const canUpgrade =
              previewsCurrent && upgrade !== undefined && canAffordUpgrade && !financeBlockReason;
            const isLoading = actionLoading === facility.id;
            const upgradeReason = financeBlockReason
              ? resolveBackendError(financeBlockReason)
              : facilityUpgradeError?.facilityId === facility.id
                ? facilityUpgradeError.message
                : null;

            return (
              <div
                key={facility.id}
                className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 flex flex-col gap-4"
              >
                <div className="space-y-1">
                  <h3 className="font-heading font-bold text-base text-gray-900 dark:text-gray-100 uppercase tracking-wider">
                    {t(facility.titleKey)}
                  </h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {t("finances.facilityLevel", { level })}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {t(facility.effectKey)}
                  </p>
                </div>

                <div className="space-y-2 mt-auto">
                  {upgrade && (
                    <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {t("finances.nextUpgradeCost", {
                        amount: formatExactMoney(upgrade.nextCost),
                      })}
                    </p>
                  )}
                  <Button
                    disabled={!canUpgrade || isLoading}
                    onClick={() => onUpgrade(facility.id)}
                    size="sm"
                  >
                    {t("finances.upgradeFacility")}
                  </Button>
                  {previewsCurrent && !canAffordUpgrade && !upgradeReason && (
                    <p className="text-xs text-red-500 dark:text-red-400">
                      {t("finances.insufficientFunds")}
                    </p>
                  )}
                  {upgradeReason && (
                    <p className="text-xs text-red-500 dark:text-red-400">{upgradeReason}</p>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </CardBody>
    </Card>
  );
}
