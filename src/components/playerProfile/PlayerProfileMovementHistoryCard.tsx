import type { PlayerMovementEntry, PlayerMovementKind, ReleaseReason } from "../../store/gameStore";
import { formatExactMoney } from "../../lib/helpers";
import { Badge, Card, CardBody, CardHeader } from "../ui";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileMovementHistoryCardProps {
  movementHistory: PlayerMovementEntry[];
  t: TranslateFn;
}

const MOVEMENT_LABEL_KEYS: Record<PlayerMovementKind, string> = {
  permanent_transfer: "playerProfile.movementPermanentTransfer",
  loan_start: "playerProfile.movementLoanStart",
  loan_return: "playerProfile.movementLoanReturn",
  loan_to_buy: "playerProfile.movementLoanToBuy",
  free_agent_signing: "playerProfile.movementFreeAgentSigning",
  released: "playerProfile.movementReleased",
  renewal: "playerProfile.movementRenewal",
  retired: "playerProfile.movementRetired",
  initial_contract: "playerProfile.movementInitialContract",
};

const RELEASE_REASON_KEYS: Record<ReleaseReason, string> = {
  expired: "playerProfile.movementReleasedExpired",
  terminated: "playerProfile.movementReleasedTerminated",
};

const MOVEMENT_BADGE_VARIANTS: Record<
  PlayerMovementKind,
  "neutral" | "success" | "primary" | "danger"
> = {
  permanent_transfer: "primary",
  loan_start: "neutral",
  loan_return: "neutral",
  loan_to_buy: "success",
  free_agent_signing: "success",
  released: "danger",
  renewal: "success",
  retired: "neutral",
  initial_contract: "neutral",
};

function movementDirection(entry: PlayerMovementEntry, t: TranslateFn): string {
  const fromName = entry.from_team_name || entry.from_team_id || "";
  const toName = entry.to_team_name || entry.to_team_id || "";

  if (fromName && toName) {
    return t("playerProfile.movementFromTo", {
      from: fromName,
      to: toName,
    });
  }

  if (toName) {
    return t("playerProfile.movementTo", { to: toName });
  }

  if (fromName) {
    return t("playerProfile.movementFrom", { from: fromName });
  }

  return "";
}

/** The contract's span in words. An unknown start is left out, never replaced by a date. */
function contractSpan(entry: PlayerMovementEntry, t: TranslateFn): string | null {
  const contract = entry.contract;
  if (!contract) return null;
  if (contract.start && contract.end) {
    return t("playerProfile.movementContractRange", { start: contract.start, end: contract.end });
  }
  if (contract.end) {
    return t("playerProfile.movementContractUntil", { end: contract.end });
  }
  if (contract.start) {
    return t("playerProfile.movementContractFrom", { start: contract.start });
  }
  return null;
}

export default function PlayerProfileMovementHistoryCard({
  movementHistory,
  t,
}: PlayerProfileMovementHistoryCardProps) {
  // Newest first. Entries on the same day keep the order they happened in, which is
  // the order they were recorded, so the later one is listed above the earlier.
  const sortedHistory = movementHistory
    .map((entry, index) => ({ entry, index }))
    .sort(
      (left, right) => right.entry.date.localeCompare(left.entry.date) || right.index - left.index,
    )
    .map(({ entry }) => entry);

  return (
    <Card>
      <CardHeader>{t("playerProfile.movementHistory")}</CardHeader>
      <CardBody>
        {sortedHistory.length > 0 ? (
          <div className="flex flex-col gap-3">
            {sortedHistory.map((entry, index) => {
              const direction = movementDirection(entry, t);
              const span = contractSpan(entry, t);

              return (
                <div
                  key={`${entry.date}-${entry.kind}-${index}`}
                  className="rounded-lg border border-gray-100 bg-gray-50/60 p-3 text-sm dark:border-navy-600 dark:bg-navy-800/50"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant={MOVEMENT_BADGE_VARIANTS[entry.kind]}>
                      {t(MOVEMENT_LABEL_KEYS[entry.kind])}
                    </Badge>
                    <span className="text-xs font-semibold text-gray-500 dark:text-gray-400">
                      {entry.date || t("playerProfile.movementDateUnknown")}
                    </span>
                  </div>

                  {direction ? (
                    <div className="mt-2 font-semibold text-gray-800 dark:text-gray-100">
                      {direction}
                    </div>
                  ) : null}

                  <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-gray-500 dark:text-gray-400">
                    {entry.fee ? (
                      <span>
                        {t("playerProfile.movementFee", {
                          fee: formatExactMoney(entry.fee),
                        })}
                      </span>
                    ) : null}
                    {entry.loan_end_date ? (
                      <span>
                        {t("playerProfile.movementLoanUntil", {
                          date: entry.loan_end_date,
                        })}
                      </span>
                    ) : null}
                    {span ? <span>{span}</span> : null}
                    {entry.contract ? (
                      <span>
                        {t("playerProfile.movementWage", {
                          wage: formatExactMoney(entry.contract.weekly_wage),
                        })}
                      </span>
                    ) : null}
                    {entry.contract?.source === "legacy_migrated" ? (
                      <span>{t("playerProfile.movementLegacyContract")}</span>
                    ) : null}
                    {entry.release_reason ? (
                      <span>{t(RELEASE_REASON_KEYS[entry.release_reason])}</span>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <p className="py-4 text-center text-sm text-gray-400 dark:text-gray-500">
            {t("playerProfile.noMovementHistory")}
          </p>
        )}
      </CardBody>
    </Card>
  );
}
