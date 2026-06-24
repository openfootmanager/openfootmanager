import type { FixtureData, TeamData } from "../../store/gameStore";
import type { MatchModeType } from "../../hooks/useAdvanceTime";
import type { BlockerModal } from "../../hooks/useAdvanceTime.helpers";
import type { DigestEntry, DigestStopReason } from "../../hooks/useDigestAdvance";
import DashboardBlockerModal from "./DashboardBlockerModal";
import DashboardCloseConfirmModal from "./DashboardCloseConfirmModal";
import DashboardExitConfirmModal from "./DashboardExitConfirmModal";
import DashboardExitSavingModal from "./DashboardExitSavingModal";
import { type DashboardMatchModeMeta } from "./DashboardHeader";
import DashboardMatchConfirmModal from "./DashboardMatchConfirmModal";
import DashboardResultsRecapModal from "./DashboardResultsRecapModal";
import DashboardSimulatingModal from "./DashboardSimulatingModal";
import type { AdvanceRecap } from "./advanceRecap";

interface DashboardOverlaysProps {
  blockerModal: BlockerModal | null;
  currentModeMeta: DashboardMatchModeMeta;
  isAdvancing: boolean;
  recapResults: AdvanceRecap | null;
  onCloseRecap: () => void;
  handleConfirmMatch: () => void;
  handleExitToMenu: () => void | Promise<void>;
  handleNavigate: (tab: string) => void;
  handleCloseQuit: (save: boolean) => void | Promise<void>;
  isExitingToMenu: boolean;
  matchMode: MatchModeType;
  setBlockerModal: (value: BlockerModal | null) => void;
  setShowCloseConfirm: (value: boolean) => void;
  setShowExitConfirm: (value: boolean) => void;
  setShowMatchConfirm: (value: boolean) => void;
  showCloseConfirm: boolean;
  showExitConfirm: boolean;
  showMatchConfirm: boolean;
  teams: TeamData[];
  todayMatchFixture: FixtureData | null;
  // Digest feed props (present when digest mode is active)
  digestEntries?: DigestEntry[];
  digestStopReason?: DigestStopReason | null;
  isDigestVisible?: boolean;
  isDigestRunning?: boolean;
  isDigestAborting?: boolean;
  onDigestContinueAfterBlocker?: () => void;
  onDismissDigest?: () => void;
  onDigestStop?: () => void;
}

export default function DashboardOverlays({
  blockerModal,
  currentModeMeta,
  isAdvancing,
  recapResults,
  onCloseRecap,
  handleConfirmMatch,
  handleExitToMenu,
  handleNavigate,
  handleCloseQuit,
  isExitingToMenu,
  matchMode,
  setBlockerModal,
  setShowCloseConfirm,
  setShowExitConfirm,
  setShowMatchConfirm,
  showCloseConfirm,
  showExitConfirm,
  showMatchConfirm,
  teams,
  todayMatchFixture,
  digestEntries,
  digestStopReason,
  isDigestVisible,
  isDigestRunning,
  isDigestAborting,
  onDigestContinueAfterBlocker,
  onDismissDigest,
  onDigestStop,
}: DashboardOverlaysProps) {
  return (
    <>
      {(isDigestVisible || isAdvancing) ? (
        <DashboardSimulatingModal
          digestEntries={digestEntries}
          isDigestRunning={isDigestRunning}
          isDigestAborting={isDigestAborting}
          stopReason={digestStopReason}
          onStop={onDigestStop}
          onDismiss={onDismissDigest}
          onNavigate={handleNavigate}
          onContinueAfterBlocker={onDigestContinueAfterBlocker}
        />
      ) : null}

      {!isAdvancing && recapResults ? (
        <DashboardResultsRecapModal recap={recapResults} onClose={onCloseRecap} />
      ) : null}

      {isExitingToMenu ? <DashboardExitSavingModal /> : null}

      {showExitConfirm ? (
        <DashboardExitConfirmModal
          onCancel={() => setShowExitConfirm(false)}
          onConfirm={() => {
            setShowExitConfirm(false);
            void handleExitToMenu();
          }}
        />
      ) : null}

      {showCloseConfirm ? (
        <DashboardCloseConfirmModal
          onCancel={() => setShowCloseConfirm(false)}
          onQuitWithoutSave={() => void handleCloseQuit(false)}
          onSaveAndQuit={() => void handleCloseQuit(true)}
        />
      ) : null}

      {showMatchConfirm ? (
        <DashboardMatchConfirmModal
          matchMode={matchMode}
          modeMeta={currentModeMeta}
          onCancel={() => setShowMatchConfirm(false)}
          onConfirm={handleConfirmMatch}
          teams={teams}
          todayMatchFixture={todayMatchFixture}
        />
      ) : null}

      {blockerModal ? (
        <DashboardBlockerModal
          blockerModal={blockerModal}
          onClose={() => setBlockerModal(null)}
          onContinueAnyway={blockerModal.pendingAction ?? null}
          onNavigate={(tab) => {
            setBlockerModal(null);
            handleNavigate(tab);
          }}
        />
      ) : null}
    </>
  );
}