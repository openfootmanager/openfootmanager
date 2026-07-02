import type { PlayerMovementEntry, PlayerMovementKind } from "../../store/gameStore";
import { formatExactMoney } from "../../lib/helpers";
import { Badge, Card, CardBody, CardHeader } from "../ui";

type TranslateFn = (
    key: string,
    options?: Record<string, string | number>,
) => string;

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
};

const MOVEMENT_BADGE_VARIANTS: Record<PlayerMovementKind, "neutral" | "success" | "primary" | "danger"> = {
    permanent_transfer: "primary",
    loan_start: "neutral",
    loan_return: "neutral",
    loan_to_buy: "success",
    free_agent_signing: "success",
    released: "danger",
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

export default function PlayerProfileMovementHistoryCard({
    movementHistory,
    t,
}: PlayerProfileMovementHistoryCardProps) {
    const sortedHistory = [...movementHistory].sort((left, right) =>
        right.date.localeCompare(left.date),
    );

    return (
        <Card>
            <CardHeader>{t("playerProfile.movementHistory")}</CardHeader>
            <CardBody>
                {sortedHistory.length > 0 ? (
                    <div className="flex flex-col gap-3">
                        {sortedHistory.map((entry, index) => {
                            const direction = movementDirection(entry, t);

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
                                            {entry.date}
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
