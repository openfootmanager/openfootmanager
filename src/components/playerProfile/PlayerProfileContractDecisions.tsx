import { formatExactMoney } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { Button, Card, CardBody, CardHeader } from "../ui";
import { terminationBlockedBy } from "./PlayerProfile.contract";
import { formatPlayerWage, formatProfileDay } from "./PlayerProfile.helpers";
import { Decision, Panel, Pending, SECTION, type TranslateFn } from "./contractSheetParts";
import type { ContractSheetData } from "./useContractSheetData";

interface PlayerProfileContractDecisionsProps {
  player: PlayerData;
  hasLetExpireIntent: boolean;
  /** His weekly wage when the club may see it exactly; `null` otherwise. */
  exactWeeklyWage: number | null;
  language: string;
  data: ContractSheetData;
  actionSubmitting: boolean;
  onOpenRenewal: () => void;
  onMarkLetExpire: () => void;
  onClearLetExpire: () => void;
  onOpenTermination: () => void;
  t: TranslateFn;
}

/**
 * "What you can do": Renew, Let expire and Terminate side by side, each with what it does and a
 * panel of what it costs before the click. All three stay when the contract is set to expire: Renew
 * then says the talks are closed and offers to reopen them, and Let expire shows the day he leaves.
 */
export default function PlayerProfileContractDecisions(props: PlayerProfileContractDecisionsProps) {
  const { hasLetExpireIntent, t } = props;
  return (
    <section aria-label={t(`${SECTION}.decide`)}>
      <Card>
        <CardHeader>{t(`${SECTION}.decide`)}</CardHeader>
        <CardBody>
          <div className="@container">
            <div className="grid grid-cols-1 gap-x-4 gap-y-6 @[34rem]:grid-cols-3 @[34rem]:grid-rows-[auto_1fr_auto_auto] @[34rem]:gap-y-2">
              {hasLetExpireIntent ? <ReopenTalks {...props} /> : <Renew {...props} />}
              <LetExpire {...props} />
              <Terminate {...props} />
            </div>
          </div>
        </CardBody>
      </Card>
    </section>
  );
}

function Renew({
  player,
  exactWeeklyWage,
  language,
  data,
  onOpenRenewal,
  t,
}: PlayerProfileContractDecisionsProps) {
  const contractEnd = player.contract_end;
  const endText = contractEnd ? formatProfileDay(contractEnd, language) : "";
  const projection = data.projection.status === "answered" ? data.projection.value : null;
  return (
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
      <Button size="sm" className="w-full" disabled={!contractEnd} onClick={onOpenRenewal}>
        {t("common.renewContract")}
      </Button>
    </Decision>
  );
}

/** Renew while the contract is set to expire: the talks are closed, and reopening them is the action. */
function ReopenTalks({
  player,
  exactWeeklyWage,
  language,
  actionSubmitting,
  onClearLetExpire,
  t,
}: PlayerProfileContractDecisionsProps) {
  const contractEnd = player.contract_end;
  const endText = contractEnd ? formatProfileDay(contractEnd, language) : "";
  return (
    <Decision
      title={t("common.renewContract")}
      what={t(`${SECTION}.renewClosedWhat`)}
      panel={
        <Panel label={t(`${SECTION}.now`)}>
          {exactWeeklyWage !== null && contractEnd
            ? t(`${SECTION}.nowValue`, { wage: formatPlayerWage(exactWeeklyWage), date: endText })
            : endText}
        </Panel>
      }
    >
      <Button
        variant="outline"
        size="sm"
        className="w-full"
        disabled={actionSubmitting}
        onClick={onClearLetExpire}
      >
        {t("playerProfile.reopenContractTalks")}
      </Button>
    </Decision>
  );
}

function LetExpire({
  player,
  hasLetExpireIntent,
  language,
  actionSubmitting,
  onMarkLetExpire,
  t,
}: PlayerProfileContractDecisionsProps) {
  const contractEnd = player.contract_end;
  const endText = contractEnd ? formatProfileDay(contractEnd, language) : "";
  return (
    <Decision
      title={t("playerProfile.letContractExpire")}
      what={t(`${SECTION}.${hasLetExpireIntent ? "letExpireSetWhat" : "letExpireWhat"}`)}
      panel={
        <Panel label={t(`${SECTION}.${hasLetExpireIntent ? "leavesOn" : "leaveOn"}`)}>
          {endText}
        </Panel>
      }
    >
      <Button
        variant="outline"
        size="sm"
        className="w-full"
        disabled={actionSubmitting || !contractEnd || hasLetExpireIntent}
        onClick={onMarkLetExpire}
      >
        {t(
          hasLetExpireIntent ? `${SECTION}.letExpireSetButton` : "playerProfile.letContractExpire",
        )}
      </Button>
    </Decision>
  );
}

function Terminate({
  player,
  data,
  actionSubmitting,
  onOpenTermination,
  t,
}: PlayerProfileContractDecisionsProps) {
  const blocked = terminationBlockedBy(player);
  const termination = data.termination.status === "answered" ? data.termination.value : null;
  const squadUnsafe = termination !== null && !termination.squad_safety.can_field_matchday_squad;
  const panel =
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
  return (
    <Decision
      title={t("playerProfile.terminateContract")}
      what={t(`${SECTION}.terminateWhat`)}
      panel={
        <>
          {panel}
          {squadUnsafe ? (
            <p className="text-body text-danger">{t("playerProfile.terminationUnsafe")}</p>
          ) : null}
        </>
      }
    >
      <Button
        variant="outline"
        size="sm"
        className="w-full border-danger hover:bg-danger-tint hover:border-danger"
        disabled={actionSubmitting || blocked !== null || squadUnsafe}
        onClick={onOpenTermination}
      >
        <span className="text-danger">{t("playerProfile.terminateContract")}</span>
      </Button>
    </Decision>
  );
}
