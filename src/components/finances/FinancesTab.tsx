import { useFinanceView } from "./useFinanceView";
import { FinanceWageSummary } from "./FinanceWageSummary";
import { FinanceCashFlowCard } from "./FinanceCashFlowCard";
import { FinanceContractRiskCard } from "./FinanceContractRiskCard";
import { FinanceSponsorsCard } from "./FinanceSponsorsCard";
import { FinanceOverview } from "./FinanceOverview";
import { useEffect, useState } from "react";

import type { GameStateData, PlayerSelectionOptions, TeamData } from "../../store/gameStore";
import { Button } from "../ui";

import { getFinanceSnapshot, type FinanceSnapshotData } from "../../services/financeService";

import { useTranslation } from "react-i18next";

import { type TaggedFinanceSnapshotData, buildFinanceSnapshotKey } from "./FinancesTab.helpers";
import FinancesFacilitiesCard from "./FinancesFacilitiesCard";
import FinancesPayrollTable from "./FinancesPayrollTable";

interface FinancesTabProps {
  gameState: GameStateData;
  onGameUpdate?: (state: GameStateData) => void;
  onSelectPlayer?: (id: string, options?: PlayerSelectionOptions) => void;
}

/**
 * Resolves the managed team, and renders the tab only once there is one.
 *
 * The guard needs its own component so that each component's hook sequence is
 * the same on every render. Here `useTranslation` runs before the early return,
 * so this component always contributes exactly that one hook whether or not a
 * team was found; `FinancesTabContent` is mounted only with a team in hand, so
 * it always runs all of its own hooks. Putting the guard directly above
 * `FinancesTabContent`'s ten `useState` calls instead would take the hook count
 * from one to eleven the moment the manager's team resolved, which React
 * rejects with "Rendered more hooks than during the previous render".
 *
 * The two components can be merged again once the tab no longer needs state.
 */
export default function FinancesTab({ gameState, onGameUpdate, onSelectPlayer }: FinancesTabProps) {
  const { t } = useTranslation();
  const myTeam = gameState.teams.find((tm) => tm.id === gameState.manager.team_id);
  if (!myTeam) return <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>;

  return (
    <FinancesTabContent
      gameState={gameState}
      myTeam={myTeam}
      onGameUpdate={onGameUpdate}
      onSelectPlayer={onSelectPlayer}
    />
  );
}

interface FinancesTabContentProps extends FinancesTabProps {
  myTeam: TeamData;
}

/**
 * Shows the finances the backend computed, and nothing else.
 *
 * The tab used to fall back to a TypeScript projection while the backend snapshot was loading
 * and on every refetch, so the figures visibly changed once the real ones arrived. The last
 * backend snapshot is kept across a refetch instead, and until the first one lands there is a
 * placeholder, never a guess.
 */
function FinancesTabContent({
  gameState,
  myTeam,
  onGameUpdate,
  onSelectPlayer,
}: FinancesTabContentProps) {
  const { t } = useTranslation();
  const financeSnapshotKey = buildFinanceSnapshotKey(gameState, myTeam);
  const [remoteFinanceData, setRemoteFinanceData] = useState<TaggedFinanceSnapshotData | null>(
    null,
  );
  const [loadFailure, setLoadFailure] = useState<{ teamId: string; key: string } | null>(null);
  const [loadAttempt, setLoadAttempt] = useState(0);

  // Slim session updates may omit the foreign player carrying a reservation, so a fresh
  // career state must refresh commitments even when its visible finance fields are unchanged.
  useEffect(() => {
    let cancelled = false;
    const requestKey = financeSnapshotKey;

    setLoadFailure(null);
    void getFinanceSnapshot(myTeam.id)
      .then((financeData) => {
        if (!cancelled) {
          setRemoteFinanceData({ key: requestKey, teamId: myTeam.id, data: financeData });
        }
      })
      .catch((error) => {
        console.error("Failed to load finance snapshot:", error);
        if (!cancelled) {
          setLoadFailure({ teamId: myTeam.id, key: requestKey });
        }
      });

    return () => {
      cancelled = true;
    };
  }, [financeSnapshotKey, myTeam.id, loadAttempt, gameState]);

  const ownData = remoteFinanceData?.teamId === myTeam.id ? remoteFinanceData : null;
  const failedForThisState =
    loadFailure?.teamId === myTeam.id && loadFailure.key === financeSnapshotKey;
  const failureBanner = failedForThisState ? (
    <div className="flex items-center gap-3 text-sm text-gray-500 dark:text-gray-400">
      <p role="alert">{t("finances.snapshotLoadFailed")}</p>
      <Button size="sm" variant="outline" onClick={() => setLoadAttempt((n) => n + 1)}>
        {t("common.retry")}
      </Button>
    </div>
  ) : null;

  if (!ownData) {
    return (
      failureBanner ?? <p className="text-gray-500 dark:text-gray-400">{t("common.loading")}</p>
    );
  }

  return (
    <>
      {failureBanner}
      <FinancesTabLoaded
        gameState={gameState}
        myTeam={myTeam}
        financeData={ownData.data}
        previewsCurrent={ownData.key === financeSnapshotKey}
        onGameUpdate={onGameUpdate}
        onSelectPlayer={onSelectPlayer}
      />
    </>
  );
}

interface FinancesTabLoadedProps extends FinancesTabContentProps {
  financeData: FinanceSnapshotData;
  /** Previews describe the club as it was when fetched; a stale set must not enable an action. */
  previewsCurrent: boolean;
}

function FinancesTabLoaded(props: FinancesTabLoadedProps) {
  const view = useFinanceView(props);
  const {
    myTeam,
    financeSnapshot,
    totalValue,
    facilities,
    financeData,
    previewsCurrent,
    facilityUpgradeError,
    actionLoading,
    handleUpgradeFacility,
    roster,
    onSelectPlayer,
  } = view;
  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
      <FinanceOverview myTeam={myTeam} snapshot={financeSnapshot} totalValue={totalValue} />

      {/* Wage summary */}
      <FinanceWageSummary state={view} />

      <FinanceCashFlowCard state={view} />

      <FinanceContractRiskCard state={view} />

      <FinanceSponsorsCard state={view} />

      <FinancesFacilitiesCard
        facilities={facilities}
        facilityUpgrades={financeData.previews.facilityUpgrades}
        previewsCurrent={previewsCurrent}
        facilityUpgradeError={facilityUpgradeError}
        actionLoading={actionLoading}
        onUpgrade={(facility) => void handleUpgradeFacility(facility)}
      />

      <FinancesPayrollTable roster={roster} onSelectPlayer={onSelectPlayer} />
    </div>
  );
}
