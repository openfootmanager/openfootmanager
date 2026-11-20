import { Card, CardHeader, CardBody, Button } from "../ui";
import { formatExactMoney, formatWeeklyWage } from "../../lib/helpers";

import type { FinanceViewState } from "./useFinanceView";
import {
  ENVELOPE_TRANSFER_STEP,
  ENVELOPE_WAGE_STEP_WEEKLY,
  canRaiseTransferBudget,
  canRaiseWageBudget,
} from "./FinanceEnvelopeCard.helpers";

export function FinanceEnvelopeCard({
  state,
}: {
  state: Pick<
    FinanceViewState,
    | "t"
    | "financeSnapshot"
    | "myTeam"
    | "previewsCurrent"
    | "reallocationLoading"
    | "handleReallocateBoardEnvelopes"
    | "reallocationFeedback"
  >;
}) {
  const {
    t,
    financeSnapshot,
    myTeam,
    previewsCurrent,
    reallocationLoading,
    handleReallocateBoardEnvelopes,
    reallocationFeedback,
  } = state;
  const busy = reallocationLoading;
  const wageBudget = myTeam.wage_budget;
  const transferBudget = myTeam.transfer_budget;
  const raiseWageEnabled =
    previewsCurrent && canRaiseWageBudget(transferBudget, financeSnapshot.committedTransferFees);
  const raiseTransferEnabled = canRaiseTransferBudget(wageBudget);

  return (
    <Card>
      <CardHeader>{t("finances.reallocateEnvelopes")}</CardHeader>
      <CardBody>
        <p className="text-body text-fg-muted">{t("finances.reallocateEnvelopesDescription")}</p>
        <dl className="mt-3 grid grid-cols-2 gap-3">
          <div>
            <dt className="text-caption font-heading font-bold text-fg-muted">
              {t("finances.wageBudget")}
            </dt>
            <dd className="font-heading font-bold text-heading text-fg tabular-nums">
              {formatWeeklyWage(wageBudget)}
            </dd>
          </div>
          <div>
            <dt className="text-caption font-heading font-bold text-fg-muted">
              {t("finances.transferBudget")}
            </dt>
            <dd className="font-heading font-bold text-heading text-fg tabular-nums">
              {formatExactMoney(transferBudget)}
            </dd>
          </div>
        </dl>
        <p className="mt-3 text-caption text-fg-muted">
          {t("finances.reallocateStep", {
            wage: formatWeeklyWage(ENVELOPE_WAGE_STEP_WEEKLY),
            transfer: formatExactMoney(ENVELOPE_TRANSFER_STEP),
          })}
        </p>
        <div className="mt-4 flex flex-col gap-2 sm:flex-row">
          <Button
            variant="outline"
            size="sm"
            disabled={!raiseWageEnabled || busy}
            onClick={() => void handleReallocateBoardEnvelopes(ENVELOPE_WAGE_STEP_WEEKLY)}
          >
            {t("finances.raiseWageBudget")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={!raiseTransferEnabled || busy}
            onClick={() => void handleReallocateBoardEnvelopes(-ENVELOPE_WAGE_STEP_WEEKLY)}
          >
            {t("finances.raiseTransferBudget")}
          </Button>
        </div>
        {reallocationFeedback ? (
          <p
            className={`mt-3 text-body ${reallocationFeedback.tone === "error" ? "text-danger" : "text-positive"}`}
            role="status"
          >
            {reallocationFeedback.text}
          </p>
        ) : null}
      </CardBody>
    </Card>
  );
}
