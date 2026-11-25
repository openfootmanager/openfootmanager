import { getContractRiskBadgeVariant } from "../../lib/contractUtils";
import type { PlayerData } from "../../store/gameStore";
import { Badge, Card, CardBody, CardHeader } from "../ui";
import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import { formatPlayerWage, formatProfileDay, formatYearsLeft } from "./PlayerProfile.helpers";
import { Row, SECTION, type TranslateFn } from "./contractSheetParts";

interface PlayerProfileContractSummaryProps {
  player: PlayerData;
  /** The game's date, for the years left on his contract. */
  currentDate: string;
  isOwnContract: boolean;
  /** His weekly wage when the club may see it exactly; `null` for every other state. */
  exactWeeklyWage: number | null;
  language: string;
  /** The backend's rating of how near his contract's end is. */
  contractRisk: { level: "critical" | "warning" | "stable"; label: string };
  /** The contract he is on now, from his history; `null` when he has none. */
  lastSigned: ContractHistoryRow | null;
  t: TranslateFn;
}

/** "His contract": when it ends, the years left and their risk, his wage, and when he last signed. */
export default function PlayerProfileContractSummary({
  player,
  currentDate,
  isOwnContract,
  exactWeeklyWage,
  language,
  contractRisk,
  lastSigned,
  t,
}: PlayerProfileContractSummaryProps) {
  const contractEnd = player.contract_end;
  return (
    <Card>
      <CardHeader>{t(`${SECTION}.his`)}</CardHeader>
      <CardBody>
        {contractEnd ? (
          <dl className="flex flex-col divide-y divide-line">
            <Row label={t(`${SECTION}.ends`)}>{formatProfileDay(contractEnd, language)}</Row>
            <Row label={t(`${SECTION}.timeLeft`)}>
              <span className="inline-flex items-center gap-2">
                {formatYearsLeft(contractEnd, currentDate, language, t)}
                <Badge variant={getContractRiskBadgeVariant(contractRisk.level)}>
                  {contractRisk.label}
                </Badge>
              </span>
            </Row>
            {isOwnContract && exactWeeklyWage !== null ? (
              <Row label={t(`${SECTION}.wage`)}>{formatPlayerWage(exactWeeklyWage)}</Row>
            ) : null}
            {lastSigned ? (
              <Row label={t(`${SECTION}.lastSigned`)}>
                {lastSigned.source === "legacy_migrated" || !lastSigned.start
                  ? t("playerProfile.movementLegacyContract")
                  : t(`${SECTION}.lastSignedValue`, {
                      date: formatProfileDay(lastSigned.start, language),
                      source: t(`${SECTION}.source.${lastSigned.source}`),
                    })}
              </Row>
            ) : null}
          </dl>
        ) : (
          <p className="text-body text-fg-muted">{t(`${SECTION}.noContract`)}</p>
        )}
      </CardBody>
    </Card>
  );
}
