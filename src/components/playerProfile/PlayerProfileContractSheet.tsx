import type { ReactNode } from "react";

import { formatExactMoney, getContractYearsRemaining } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { Button, Card, CardBody, CardHeader } from "../ui";
import { terminationBlockedBy } from "./PlayerProfile.contract";
import { formatPlayerWage } from "./PlayerProfile.helpers";
import ProfileLoadFailed from "./ProfileLoadFailed";
import type { ContractSheetData, SheetAnswer } from "./useContractSheetData";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

export interface PlayerProfileContractSheetProps {
  player: PlayerData;
  /** The game's date, for the years left on his contract. */
  currentDate: string;
  /** The manager holds his contract (his own player, or the parent club's player on loan). */
  isOwnContract: boolean;
  isFreeAgent: boolean;
  hasLetExpireIntent: boolean;
  /** His weekly wage when the club may see it exactly; `null` for every other state. */
  exactWeeklyWage: number | null;
  data: ContractSheetData;
  actionSubmitting: boolean;
  onOpenRenewal: () => void;
  onMarkLetExpire: () => void;
  onClearLetExpire: () => void;
  onOpenTermination: () => void;
  onOpenFreeAgentContract: () => void;
  t: TranslateFn;
}

const SECTION = "playerProfile.contractSheet";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4">
      <dt className="text-body text-fg-muted">{label}</dt>
      <dd className="text-body font-semibold text-fg tabular-nums">{children}</dd>
    </div>
  );
}

/** A decision: what it does and costs, written before the button that takes it. */
function Decision({
  title,
  note,
  extra,
  children,
}: {
  title: string;
  note: ReactNode;
  extra?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-line bg-sunken p-3">
      <h4 className="font-heading text-lead font-bold text-fg">{title}</h4>
      <p className="text-body text-fg-muted">{note}</p>
      {extra}
      <div>{children}</div>
    </div>
  );
}

/** A preview that is still loading or failed in place of its figures; null once it is answered. */
function Pending<T>({
  answer,
  retry,
  t,
}: {
  answer: SheetAnswer<T>;
  retry: () => void;
  t: TranslateFn;
}) {
  if (answer.status === "loading")
    return <p className="text-body text-fg-subtle">{t("common.loading")}</p>;
  if (answer.status === "failed") return <ProfileLoadFailed t={t} onRetry={retry} />;
  return null;
}

/**
 * The Contract tab's decision sheet: his contract, the club's wage bill, and what each decision
 * does and costs before it is taken. The previews load with the tab; his own player's contract
 * only, so another club's player shows dates and nothing to act on, and a free agent shows an offer.
 *
 * The wage bill and budget are weekly amounts although the backend names them "annual" (#300).
 */
export default function PlayerProfileContractSheet({
  player,
  currentDate,
  isOwnContract,
  isFreeAgent,
  hasLetExpireIntent,
  exactWeeklyWage,
  data,
  actionSubmitting,
  onOpenRenewal,
  onMarkLetExpire,
  onClearLetExpire,
  onOpenTermination,
  onOpenFreeAgentContract,
  t,
}: PlayerProfileContractSheetProps) {
  if (isFreeAgent) {
    return (
      <Card>
        <CardHeader>{t(`${SECTION}.his`)}</CardHeader>
        <CardBody>
          <p className="mb-3 text-body text-fg-muted">{t(`${SECTION}.noContract`)}</p>
          <Button variant="outline" size="sm" onClick={onOpenFreeAgentContract}>
            {t("transfers.offerContract")}
          </Button>
        </CardBody>
      </Card>
    );
  }

  const contractEnd = player.contract_end;
  const projection = data.projection.status === "answered" ? data.projection.value : null;
  const budgetUse =
    projection && projection.annual_wage_budget > 0
      ? Math.round((projection.current_weekly_wage_spend / projection.annual_wage_budget) * 100)
      : null;
  const share =
    projection && exactWeeklyWage !== null && projection.current_weekly_wage_spend > 0
      ? Math.round((exactWeeklyWage / projection.current_weekly_wage_spend) * 100)
      : null;
  const runway = projection
    ? projection.current_cash_runway_weeks === null
      ? t("finances.runwayStable")
      : t("finances.runwayWeeks", { count: projection.current_cash_runway_weeks })
    : null;

  const blocked = terminationBlockedBy(player);
  const termination = data.termination.status === "answered" ? data.termination.value : null;
  const squadUnsafe = termination !== null && !termination.squad_safety.can_field_matchday_squad;

  const contract = (
    <Card>
      <CardHeader>{t(`${SECTION}.his`)}</CardHeader>
      <CardBody>
        {contractEnd ? (
          <dl className="flex flex-col gap-2">
            <Row label={t(`${SECTION}.ends`)}>{contractEnd}</Row>
            <Row label={t("playerProfile.yearsRemaining")}>
              {getContractYearsRemaining(contractEnd, currentDate)}
            </Row>
            {isOwnContract && exactWeeklyWage !== null ? (
              <Row label={t(`${SECTION}.weeklyWage`)}>{formatPlayerWage(exactWeeklyWage)}</Row>
            ) : null}
            {share !== null ? (
              <p className="text-caption text-fg-muted">
                {t(`${SECTION}.share`, { percent: share })}
              </p>
            ) : null}
          </dl>
        ) : (
          <p className="text-body text-fg-muted">{t(`${SECTION}.noContract`)}</p>
        )}
      </CardBody>
    </Card>
  );

  if (!isOwnContract) return contract;

  const bill = (
    <Card>
      <CardHeader>{t(`${SECTION}.clubBill`)}</CardHeader>
      <CardBody>
        <Pending answer={data.projection} retry={data.retry} t={t} />
        {projection ? (
          <div className="flex flex-col gap-1">
            <p className="text-body font-semibold text-fg">
              {t(`${SECTION}.billWeekly`, {
                amount: formatPlayerWage(projection.current_weekly_wage_spend),
              })}
            </p>
            {budgetUse !== null ? (
              <p className="text-body text-fg-muted">
                {t(`${SECTION}.budgetUse`, { percent: budgetUse })}
              </p>
            ) : null}
            {runway ? (
              <p className="text-body text-fg-muted">{t(`${SECTION}.runway`, { runway })}</p>
            ) : null}
          </div>
        ) : null}
      </CardBody>
    </Card>
  );

  const renew = (
    <Decision
      title={t("common.renewContract")}
      note={
        projection && exactWeeklyWage !== null
          ? t(`${SECTION}.renewNote`, {
              wage: formatPlayerWage(exactWeeklyWage),
              bill: formatPlayerWage(projection.projected_weekly_wage_spend),
            })
          : t(`${SECTION}.renewNoteShort`)
      }
    >
      <Button variant="outline" size="sm" disabled={!contractEnd} onClick={onOpenRenewal}>
        {t("common.renewContract")}
      </Button>
    </Decision>
  );

  const letExpire = hasLetExpireIntent ? (
    <Decision
      title={t("playerProfile.reopenContractTalks")}
      note={t(`${SECTION}.letExpireSetNote`, { date: contractEnd ?? "" })}
    >
      <Button variant="outline" size="sm" disabled={actionSubmitting} onClick={onClearLetExpire}>
        {t("playerProfile.reopenContractTalks")}
      </Button>
    </Decision>
  ) : (
    <Decision
      title={t("playerProfile.letContractExpire")}
      note={t(`${SECTION}.letExpireNote`, { date: contractEnd ?? "" })}
    >
      <Button
        variant="outline"
        size="sm"
        disabled={actionSubmitting || !contractEnd}
        onClick={onMarkLetExpire}
      >
        {t("playerProfile.letContractExpire")}
      </Button>
    </Decision>
  );

  const terminateNote =
    blocked === "no_contract"
      ? t(`${SECTION}.terminateNoContract`)
      : blocked === "on_loan"
        ? t(`${SECTION}.terminateOnLoan`)
        : termination
          ? t(`${SECTION}.terminateNote`, {
              severance: formatExactMoney(termination.severance_cost),
              healthy: termination.squad_safety.healthy_players,
              needed: termination.squad_safety.effective_xi_size,
            })
          : null;
  const terminate = (
    <Decision
      title={t("playerProfile.terminateContract")}
      note={terminateNote ?? ""}
      extra={
        <>
          {blocked === null ? <Pending answer={data.termination} retry={data.retry} t={t} /> : null}
          {squadUnsafe ? (
            <p className="text-body text-danger">{t("playerProfile.terminationUnsafe")}</p>
          ) : null}
        </>
      }
    >
      <Button
        variant="outline"
        size="sm"
        className="hover:bg-danger-tint"
        disabled={actionSubmitting || blocked !== null || squadUnsafe}
        onClick={onOpenTermination}
      >
        <span className="text-danger">{t("playerProfile.terminateContract")}</span>
      </Button>
    </Decision>
  );

  return (
    <div className="@container">
      <div className="grid grid-cols-1 gap-5 @[40rem]:grid-cols-2">
        <div className="flex flex-col gap-5">
          {contract}
          {bill}
        </div>
        <Card>
          <CardHeader>{t(`${SECTION}.decide`)}</CardHeader>
          <CardBody>
            <div className="flex flex-col gap-3">
              {hasLetExpireIntent ? null : renew}
              {letExpire}
              {terminate}
            </div>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}
