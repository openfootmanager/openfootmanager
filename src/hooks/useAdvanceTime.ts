import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { GameStateData } from "../store/gameStore";
import { useGameStore } from "../store/gameStore";
import type { BlockerModal } from "./useAdvanceTime.helpers";
import {
  advanceTimeWithMode,
  checkBlockingActions,
  skipToMatchDay,
  type SkipToMatchDayResponse,
} from "../services/advanceTimeService";
import {
  buildAdvanceRecap,
  toDatePart,
  type AdvanceRecap,
} from "../components/dashboard/advanceRecap";
import { useDigestAdvance } from "./useDigestAdvance";
import type { DigestEntry, DigestStopReason } from "./useDigestAdvance";

export type MatchModeType = "live" | "spectator" | "delegate";

export interface AdvanceTimeState {
  isAdvancing: boolean;
  showContinueMenu: boolean;
  setShowContinueMenu: (v: boolean) => void;
  showMatchConfirm: boolean;
  setShowMatchConfirm: (v: boolean) => void;
  matchMode: MatchModeType;
  setMatchMode: (v: MatchModeType) => void;
  blockerModal: BlockerModal | null;
  setBlockerModal: (v: BlockerModal | null) => void;
  recapResults: AdvanceRecap | null;
  setRecapResults: (v: AdvanceRecap | null) => void;
  handleContinue: (mode?: string) => Promise<void>;
  handleConfirmMatch: () => void;
  handleSkipToMatchDay: () => Promise<void>;
  // Digest feed state (populated when continueToNextEvent is true)
  digestEntries: DigestEntry[];
  digestStopReason: DigestStopReason | null;
  isDigestVisible: boolean;
  isDigestRunning: boolean;
  isDigestAborting: boolean;
  startDigest: () => Promise<void>;
  abortDigest: () => void;
  dismissDigest: () => void;
}

export function useAdvanceTime(
  setGameState: (state: GameStateData) => void,
  hasMatchToday: boolean,
  defaultMatchMode: MatchModeType | undefined,
  settingsLoaded: boolean,
  isUnemployed: boolean,
  continueToNextEvent: boolean = false,
): AdvanceTimeState {
  const navigate = useNavigate();
  const setShowFiredModal = useGameStore((s) => s.setShowFiredModal);
  const [isAdvancing, setIsAdvancing] = useState(false);
  const [showContinueMenu, setShowContinueMenu] = useState(false);
  const [showMatchConfirm, setShowMatchConfirm] = useState(false);
  const [matchMode, setMatchMode] = useState<MatchModeType>("live");
  const [blockerModal, setBlockerModal] = useState<BlockerModal | null>(null);
  const [recapResults, setRecapResults] = useState<AdvanceRecap | null>(null);

  const {
    isRunning: isDigestRunning,
    isAborting: isDigestAborting,
    entries: digestEntries,
    stopReason: digestStopReason,
    isVisible: isDigestVisible,
    startDigest,
    abortDigest,
    dismissDigest,
  } = useDigestAdvance(setGameState, () => setShowFiredModal(true));

  // Sync matchMode with settings when loaded
  useEffect(() => {
    if (settingsLoaded && defaultMatchMode) {
      setMatchMode(defaultMatchMode);
    }
  }, [settingsLoaded, defaultMatchMode]);

  function resetTransientUi(options?: {
    showContinueMenu?: boolean;
    showMatchConfirm?: boolean;
    blockerModal?: BlockerModal | null;
  }): void {
    setShowContinueMenu(options?.showContinueMenu ?? false);
    setShowMatchConfirm(options?.showMatchConfirm ?? false);
    setBlockerModal(options?.blockerModal ?? null);
  }

  const doAdvance = async (effectiveMode: string) => {
    console.info("[useAdvanceTime] doAdvance:start", {
      effectiveMode,
      hasMatchToday,
      matchMode,
    });
    setIsAdvancing(true);
    resetTransientUi();
    // Clock date before advancing — the cursor for "what happened" in the recap.
    const sinceDate = toDatePart(
      useGameStore.getState().gameState?.clock?.current_date,
    );
    try {
      const result = await advanceTimeWithMode(effectiveMode);
      console.info("[useAdvanceTime] doAdvance:result", {
        action: result.action,
        fixtureIndex: result.fixture_index,
        mode: result.mode || effectiveMode,
        hasGame: !!result.game,
        hasSnapshot: !!result.snapshot,
      });
      if (result.action === "fired") {
        if (result.game) setGameState(result.game as GameStateData);
        setShowFiredModal(true);
      } else if (result.action === "live_match") {
        navigate("/match", {
          state: {
            fixtureIndex: result.fixture_index,
            mode: result.mode || effectiveMode,
            snapshot: result.snapshot,
          },
        });
      } else if (result.action === "advanced" && result.game) {
        const game = result.game as GameStateData;
        setGameState(game);
        setRecapResults(
          buildAdvanceRecap(game, sinceDate, result.results ?? []),
        );
      }
    } catch (err) {
      console.error("Failed to advance time:", err);
    } finally {
      console.info("[useAdvanceTime] doAdvance:complete", { effectiveMode });
      setIsAdvancing(false);
    }
  };

  const handleContinue = async (mode?: string) => {
    const effectiveMode = mode || matchMode;
    const resolvedMode = isUnemployed ? "delegate" : effectiveMode;
    console.info("[useAdvanceTime] handleContinue", {
      effectiveMode: resolvedMode,
      hasMatchToday,
      isAdvancing,
      matchMode,
      showMatchConfirm,
    });
    // If there's a match today, show confirmation modal first
    if (hasMatchToday && !showMatchConfirm) {
      console.info("[useAdvanceTime] handleContinue:showMatchConfirm", {
        effectiveMode: resolvedMode,
      });
      if (mode) setMatchMode(mode as MatchModeType);
      resetTransientUi({ showMatchConfirm: true });
      return;
    }
    if (isAdvancing) return;
    // With the opt-in setting, Continue runs the day-by-day digest loop instead
    // of the silent batch advance (unless there's a match today, handled above).
    const runContinue = continueToNextEvent
      ? () => void startDigest()
      : () => doAdvance(resolvedMode);
    const blockers = await checkBlockingActions("handleContinue");
    if (blockers.length > 0) {
      setBlockerModal({ blockers, pendingAction: runContinue });
      return;
    }
    runContinue();
  };

  const handleConfirmMatch = () => {
    console.info("[useAdvanceTime] handleConfirmMatch", { matchMode });
    doAdvance(matchMode);
  };

  const handleSkipToMatchDay = async () => {
    if (isAdvancing) return;
    console.info("[useAdvanceTime] handleSkipToMatchDay:start");
    const blockers = await checkBlockingActions("handleSkipToMatchDay");
    if (blockers.length > 0) {
      setBlockerModal({ blockers, pendingAction: doSkipToMatchDay });
      return;
    }
    doSkipToMatchDay();
  };

  // Shared driver for the multi-day advances (Skip to Match Day and the opt-in
  // smart Continue): both roll forward several days and end on a fired / blocked
  // / arrived outcome, feeding the day-by-day recap.
  const runMultiDayAdvance = async (
    run: () => Promise<SkipToMatchDayResponse>,
    label: string,
  ) => {
    setIsAdvancing(true);
    resetTransientUi();
    const sinceDate = toDatePart(
      useGameStore.getState().gameState?.clock?.current_date,
    );
    try {
      const result = await run();
      console.info(`[useAdvanceTime] ${label}:result`, {
        action: result.action,
        daysSkipped: result.days_skipped,
        blockerCount: result.blockers?.length ?? 0,
        hasGame: !!result.game,
      });
      if (result.action === "fired") {
        if (result.game) setGameState(result.game as GameStateData);
        setShowFiredModal(true);
        return;
      }
      const game = result.game as GameStateData | undefined;
      if (game) setGameState(game);
      if (result.action === "blocked" && result.blockers && result.blockers.length > 0) {
        setBlockerModal({ blockers: result.blockers });
      } else if (game) {
        setRecapResults(buildAdvanceRecap(game, sinceDate, result.results ?? []));
      }
    } catch (err) {
      console.error(`Failed to ${label}:`, err);
    } finally {
      console.info(`[useAdvanceTime] ${label}:complete`);
      setIsAdvancing(false);
    }
  };

  const doSkipToMatchDay = () =>
    runMultiDayAdvance(skipToMatchDay, "doSkipToMatchDay");

  return {
    isAdvancing,
    showContinueMenu, setShowContinueMenu,
    showMatchConfirm, setShowMatchConfirm,
    matchMode, setMatchMode,
    blockerModal, setBlockerModal,
    recapResults, setRecapResults,
    handleContinue,
    handleConfirmMatch,
    handleSkipToMatchDay,
    digestEntries,
    digestStopReason,
    isDigestVisible,
    isDigestRunning,
    isDigestAborting,
    startDigest,
    abortDigest,
    dismissDigest,
  };
}
