import type { ReactNode } from "react";

import type { PlayerData } from "../../store/gameStore";
import { Button, Card, CardBody, CardHeader } from "../ui";
import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import PlayerProfileContractDecisions from "./PlayerProfileContractDecisions";
import PlayerProfileContractSummary from "./PlayerProfileContractSummary";
import PlayerProfileWageBill from "./PlayerProfileWageBill";
import { SECTION, type TranslateFn } from "./contractSheetParts";
import type { ContractSheetData } from "./useContractSheetData";

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

/**
 * The Contract tab's decision sheet (board "Contract A"): his contract and the club's wage bill on
 * the left; what you can do on the right, and under it the contracts he has had. The previews load
 * with the tab. Only his own player's contract gets the decisions and the bill: another club's
 * player shows his contract's dates, and a free agent an offer. Each card is its own component; this
 * one lays them out.
 */
export default function PlayerProfileContractSheet(props: PlayerProfileContractSheetProps) {
  const { player, isOwnContract, isFreeAgent, below, t } = props;

  const summary = isFreeAgent ? (
    <Card>
      <CardHeader>{t(`${SECTION}.his`)}</CardHeader>
      <CardBody>
        <p className="mb-3 text-body text-fg-muted">{t(`${SECTION}.noContract`)}</p>
        <Button variant="outline" size="sm" onClick={props.onOpenFreeAgentContract}>
          {t("transfers.offerContract")}
        </Button>
      </CardBody>
    </Card>
  ) : (
    <PlayerProfileContractSummary {...props} />
  );

  if (!isOwnContract) return <Layout left={summary} right={below} />;
  return (
    <Layout
      left={
        <>
          {summary}
          <PlayerProfileWageBill {...props} isOnLoan={player.active_loan != null} />
        </>
      }
      right={
        <>
          <PlayerProfileContractDecisions {...props} />
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
