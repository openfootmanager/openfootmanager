import { Card, CardHeader, CardBody, Button } from "../ui";
import { formatExactMoney, formatWeeklyWage } from "../../lib/helpers";

import type { FinanceViewState } from "./useFinanceView";

export function FinanceEnvelopeCard({
  state,
}: {
  state: Pick<
    FinanceViewState,
    | "t"
    | "myTeam"
    | "reallocationLoading"
    | "handleReallocateBoardEnvelopes"
    | "reallocationFeedback"
    | "raiseWagePreview"
    | "raiseTransferPreview"
  >;
}) {
  const {
    t,
    myTeam,
    reallocationLoading,
    handleReallocateBoardEnvelopes,
    reallocationFeedback,
    raiseWagePreview,
    raiseTransferPreview,
  } = state;
  const busy = reallocationLoading;
  const wageBudget = myTeam.wage_budget;
  const transferBudget = myTeam.transfer_budget;
  const raiseWageEnabled = Boolean(raiseWagePreview?.allowed) && !busy;
  const raiseTransferEnabled = Boolean(raiseTransferPreview?.allowed) && !busy;
  const stepWage = Math.abs(
    raiseWagePreview?.wageDeltaWeekly ?? raiseTransferPreview?.wageDeltaWeekly ?? 0,
  );
  const stepTransfer = Math.abs(
    raiseWagePreview?.transferDelta ?? raiseTransferPreview?.transferDelta ?? 0,
  );

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
        {stepWage > 0 && stepTransfer > 0 ? (
          <p className="mt-3 text-caption text-fg-muted">
            {t("finances.reallocateStep", {
              wage: formatWeeklyWage(stepWage),
              transfer: formatExactMoney(stepTransfer),
            })}
          </p>
        ) : null}
        <div className="mt-4 flex flex-col gap-2 sm:flex-row">
          <Button
            variant="outline"
            size="sm"
            disabled={!raiseWageEnabled}
            onClick={() => {
              if (raiseWagePreview) {
                void handleReallocateBoardEnvelopes(raiseWagePreview.wageDeltaWeekly);
              }
            }}
          >
            {t("finances.raiseWageBudget")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={!raiseTransferEnabled}
            onClick={() => {
              if (raiseTransferPreview) {
                void handleReallocateBoardEnvelopes(raiseTransferPreview.wageDeltaWeekly);
              }
            }}
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
