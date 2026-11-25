import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatVal } from "../../lib/helpers";
import { getFinanceSnapshot } from "../../services/financeService";

interface TransferBudgetCardProps {
  teamId: string;
  revision: object;
}

/** Reads the backend's reservation ledger again after each career mutation. */
export function TransferBudgetCard({ teamId, revision }: TransferBudgetCardProps) {
  const { t } = useTranslation();
  const [budget, setBudget] = useState<{
    teamId: string;
    revision: object;
    available: number;
    committed: number;
  } | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    setFailed(false);
    void getFinanceSnapshot(teamId)
      .then(({ snapshot }) => {
        if (!cancelled)
          setBudget({
            teamId,
            revision,
            available: snapshot.availableTransferBudget,
            committed: snapshot.committedTransferFees,
          });
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [teamId, revision]);

  const current = budget?.teamId === teamId && budget.revision === revision ? budget : null;
  return (
    <div className="rounded-xl bg-panel px-4 py-2 text-center" aria-live="polite">
      {failed ? (
        <p className="text-body text-danger">{t("finances.snapshotLoadFailed")}</p>
      ) : (
        <dl>
          <dt className="text-caption text-fg-muted">{t("finances.availableTransferBudget")}</dt>
          <dd className="font-heading text-heading font-bold text-fg tabular-nums">
            {current ? formatVal(current.available) : t("common.loading")}
          </dd>
          <dt className="text-caption text-fg-muted">{t("finances.committedTransferFees")}</dt>
          <dd className="text-body text-fg tabular-nums">
            {current ? formatVal(current.committed) : t("common.loading")}
          </dd>
        </dl>
      )}
    </div>
  );
}
