import type { ReactNode } from "react";

import { formatExactMoney, getContractYearsRemaining } from "../../lib/helpers";
import { getContractRiskBadgeVariant } from "../../lib/contractUtils";
import type { PlayerData } from "../../store/gameStore";
import { Badge, Button, Card, CardBody, CardHeader, ProgressBar } from "../ui";
import { terminationBlockedBy } from "./PlayerProfile.contract";
import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import { formatPlayerWage, formatProfileDay } from "./PlayerProfile.helpers";
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
  /** The language dates and percentages are written in. */
  language: string;
  /** The backend's rating of how near his contract's end is. */
  contractRisk: { level: "critical" | "warning" | "stable"; label: string };
  /** The contract he is on now, from his history; `null` when he has none. */
  lastSigned: ContractHistoryRow | null;
  /** Placed in the right column under the decisions: the contracts he has had. */
  below: ReactNode;
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

/** A figure and the words for it. */
function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4">
      <dt className="shrink-0 text-body text-fg-muted">{label}</dt>
      <dd className="text-right text-body font-semibold text-fg tabular-nums">{children}</dd>
    </div>
  );
}

/** What a decision costs, written before the button that takes it. */
function Panel({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="rounded-lg border border-line bg-panel p-2">
      <p className="text-caption text-fg-muted">{label}</p>
      <div className="text-body font-semibold text-fg">{children}</div>
    </div>
  );
}

/** One decision: what it does, what it costs, and the button that takes it. */
function Decision({
  title,
  what,
  panel,
  children,
}: {
  title: string;
  what: string;
  panel: ReactNode;
  children: ReactNode;
}) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a group of related controls under one heading, named by its title
    <div role="group" aria-label={title} className="flex min-w-0 flex-col gap-2">
      <h4 className="font-heading text-lead font-bold uppercase tracking-label text-fg">{title}</h4>
      <p className="text-body text-fg-muted">{what}</p>
      <div className="mt-auto flex flex-col gap-2">
        {panel}
        <div>{children}</div>
      </div>
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
  if (answer.status === "loading") {
    return <p className="text-body text-fg-subtle">{t("common.loading")}</p>;
  }
  if (answer.status === "failed") return <ProfileLoadFailed t={t} onRetry={retry} />;
  return null;
}

/** `value` of `whole` as the language writes a percentage: a ratio of two figures the backend sent. */
function percentOf(value: number, whole: number, language: string, digits: number): string {
  return new Intl.NumberFormat(language, {
    style: "percent",
    maximumFractionDigits: digits,
  }).format(value / whole);
}

/**
 * The Contract tab's decision sheet (board "Contract A"): his contract and the club's wage bill on
 * the left; what you can do on the right, each decision with what it costs before it is taken, and
 * the contracts he has had under them. The previews load with the tab. His own player's contract
 * only gets the decisions and the bill: another club's player shows his contract's dates, and a free
 * agent an offer.
 *
 * The wage bill and budget are weekly amounts although the backend names them "annual" (#300). The
 * bar and the share are ratios of the figures the projection sends; whether the club is over budget
 * is the backend's flag, not a threshold here.
 */
export default function PlayerProfileContractSheet({
  player,
  currentDate,
  isOwnContract,
  isFreeAgent,
  hasLetExpireIntent,
  exactWeeklyWage,
  language,
  contractRisk,
  lastSigned,
  below,
  data,
  actionSubmitting,
  onOpenRenewal,
  onMarkLetExpire,
  onClearLetExpire,
  onOpenTermination,
  onOpenFreeAgentContract,
  t,
}: PlayerProfileContractSheetProps) {
  const contractEnd = player.contract_end;
  const endText = contractEnd ? formatProfileDay(contractEnd, language) : "";
  const projection = data.projection.status === "answered" ? data.projection.value : null;
  const spend = projection?.current_weekly_wage_spend ?? 0;
  const budgetRatio =
    projection && projection.annual_wage_budget > 0 ? spend / projection.annual_wage_budget : null;
  const runway = projection
    ? projection.current_cash_runway_weeks === null
      ? t("finances.runwayStable")
      : t("finances.runwayWeeks", { count: projection.current_cash_runway_weeks })
    : null;

  const left = isFreeAgent ? (
    <Card>
      <CardHeader>{t(`${SECTION}.his`)}</CardHeader>
      <CardBody>
        <p className="mb-3 text-body text-fg-muted">{t(`${SECTION}.noContract`)}</p>
        <Button variant="outline" size="sm" onClick={onOpenFreeAgentContract}>
          {t("transfers.offerContract")}
        </Button>
      </CardBody>
    </Card>
  ) : (
    <Card>
      <CardHeader>{t(`${SECTION}.his`)}</CardHeader>
      <CardBody>
        {contractEnd ? (
          <dl className="flex flex-col gap-2">
            <Row label={t(`${SECTION}.ends`)}>{endText}</Row>
            <Row label={t("playerProfile.yearsRemaining")}>
              <span className="inline-flex items-center gap-2">
                {getContractYearsRemaining(contractEnd, currentDate)}
                <Badge variant={getContractRiskBadgeVariant(contractRisk.level)}>
                  {contractRisk.label}
                </Badge>
              </span>
            </Row>
            {isOwnContract && exactWeeklyWage !== null ? (
              <Row label={t(`${SECTION}.weeklyWage`)}>{formatPlayerWage(exactWeeklyWage)}</Row>
            ) : null}
          </dl>
        ) : (
          <p className="text-body text-fg-muted">{t(`${SECTION}.noContract`)}</p>
        )}
        {lastSigned ? (
          <p className="mt-3 text-caption text-fg-muted">
            {lastSigned.source === "legacy_migrated" || !lastSigned.start
              ? t("playerProfile.movementLegacyContract")
              : t(`${SECTION}.lastSigned`, {
                  date: formatProfileDay(lastSigned.start, language),
                  source: t(`${SECTION}.source.${lastSigned.source}`),
                })}
          </p>
        ) : null}
      </CardBody>
    </Card>
  );

  if (!isOwnContract) {
    return <Layout left={left} right={below} />;
  }

  const share =
    projection && exactWeeklyWage !== null && spend > 0
      ? percentOf(exactWeeklyWage, spend, language, 1)
      : null;
  const bill = (
    <Card>
      <CardHeader>{t(`${SECTION}.clubBill`)}</CardHeader>
      <CardBody>
        <Pending answer={data.projection} retry={data.retry} t={t} />
        {projection ? (
          <dl className="flex flex-col gap-2">
            <Row label={t(`${SECTION}.billLabel`)}>{formatPlayerWage(spend)}</Row>
            {budgetRatio !== null ? (
              <div className="flex flex-col gap-1">
                <Row label={t(`${SECTION}.budgetLabel`)}>
                  {percentOf(spend, projection.annual_wage_budget, language, 0)}
                </Row>
                <ProgressBar
                  value={budgetRatio * 100}
                  variant={projection.currently_over_budget ? "danger" : "primary"}
                  label={t(`${SECTION}.budgetLabel`)}
                />
              </div>
            ) : null}
            {runway ? <Row label={t(`${SECTION}.runwayLabel`)}>{runway}</Row> : null}
            {share ? <Row label={t(`${SECTION}.shareLabel`)}>{share}</Row> : null}
          </dl>
        ) : null}
      </CardBody>
    </Card>
  );

  const blocked = terminationBlockedBy(player);
  const termination = data.termination.status === "answered" ? data.termination.value : null;
  const squadUnsafe = termination !== null && !termination.squad_safety.can_field_matchday_squad;

  const renew = (
    <Decision
      title={t("common.renewContract")}
      what={t(`${SECTION}.renewWhat`)}
      panel={
        <Panel label={t(`${SECTION}.now`)}>
          {exactWeeklyWage !== null && contractEnd
            ? t(`${SECTION}.nowValue`, { wage: formatPlayerWage(exactWeeklyWage), date: endText })
            : endText}
          {projection ? (
            <p className="text-caption font-normal text-fg-muted">
              {t(`${SECTION}.billAtThisWage`, {
                bill: formatPlayerWage(projection.projected_weekly_wage_spend),
              })}
            </p>
          ) : null}
        </Panel>
      }
    >
      <Button size="sm" disabled={!contractEnd} onClick={onOpenRenewal}>
        {t("common.renewContract")}
      </Button>
    </Decision>
  );

  const letExpire = hasLetExpireIntent ? (
    <Decision
      title={t("playerProfile.reopenContractTalks")}
      what={t(`${SECTION}.letExpireSetWhat`)}
      panel={<Panel label={t(`${SECTION}.leavesOn`)}>{endText}</Panel>}
    >
      <Button variant="outline" size="sm" disabled={actionSubmitting} onClick={onClearLetExpire}>
        {t("playerProfile.reopenContractTalks")}
      </Button>
    </Decision>
  ) : (
    <Decision
      title={t("playerProfile.letContractExpire")}
      what={t(`${SECTION}.letExpireWhat`)}
      panel={<Panel label={t(`${SECTION}.leaveOn`)}>{endText}</Panel>}
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

  const terminatePanel =
    blocked === "no_contract" ? (
      <p className="text-body text-fg-muted">{t(`${SECTION}.terminateNoContract`)}</p>
    ) : blocked === "on_loan" ? (
      <p className="text-body text-fg-muted">{t(`${SECTION}.terminateOnLoan`)}</p>
    ) : termination ? (
      <Panel label={t(`${SECTION}.severance`)}>
        {formatExactMoney(termination.severance_cost)}
        <p className="text-caption font-normal text-fg-muted">
          {t(`${SECTION}.healthyAfter`)}:{" "}
          <span className="font-semibold text-fg">
            {t(`${SECTION}.healthyValue`, {
              healthy: termination.squad_safety.healthy_players,
              needed: termination.squad_safety.effective_xi_size,
            })}
          </span>
        </p>
      </Panel>
    ) : (
      <Pending answer={data.termination} retry={data.retry} t={t} />
    );
  const terminate = (
    <Decision
      title={t("playerProfile.terminateContract")}
      what={t(`${SECTION}.terminateWhat`)}
      panel={
        <>
          {terminatePanel}
          {squadUnsafe ? (
            <p className="text-body text-danger">{t("playerProfile.terminationUnsafe")}</p>
          ) : null}
        </>
      }
    >
      <Button
        variant="outline"
        size="sm"
        className="border-danger hover:bg-danger-tint hover:border-danger"
        disabled={actionSubmitting || blocked !== null || squadUnsafe}
        onClick={onOpenTermination}
      >
        <span className="text-danger">{t("playerProfile.terminateContract")}</span>
      </Button>
    </Decision>
  );

  const decisions = (
    <section aria-label={t(`${SECTION}.decide`)}>
      <Card>
        <CardHeader>{t(`${SECTION}.decide`)}</CardHeader>
        <CardBody>
          <div className="@container">
            <div className="grid grid-cols-1 gap-4 @[34rem]:grid-cols-3">
              {hasLetExpireIntent ? null : renew}
              {letExpire}
              {terminate}
            </div>
          </div>
        </CardBody>
      </Card>
    </section>
  );

  return (
    <Layout
      left={
        <>
          {left}
          {bill}
        </>
      }
      right={
        <>
          {decisions}
          {below}
        </>
      }
    />
  );
}

/**
 * The sheet's two columns: his contract and the club's bill on the left, the decisions and the
 * contracts he has had on the right. One column below the width where the right column would be
 * cramped (container breakpoints are rem, so they follow the interface scale).
 */
function Layout({ left, right }: { left: ReactNode; right: ReactNode }) {
  return (
    <div className="@container">
      <div className="grid grid-cols-1 gap-5 @[56rem]:grid-cols-[19rem_minmax(0,1fr)]">
        <div className="flex min-w-0 flex-col gap-5">{left}</div>
        <div className="flex min-w-0 flex-col gap-5">{right}</div>
      </div>
    </div>
  );
}
