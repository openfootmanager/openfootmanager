import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { GameStateData } from "../store/gameStore";
import { useGameStore } from "../store/gameStore";
import type { BlockerModal } from "./useAdvanceTime.helpers";
import {
  advanceTimeWithMode,
  checkBlockingActions,
  skipToMatchDay,
  type AdvanceMatchResultData,
} from "../services/advanceTimeService";

export type MatchModeType = "live" | "spectator" | "delegate";

export function useAdvanceTime(
  setGameState: (state: GameStateData) => void,
  hasMatchToday: boolean,
  defaultMatchMode: MatchModeType | undefined,
  settingsLoaded: boolean,
  isUnemployed: boolean,
) {
  const navigate = useNavigate();
  const setShowFiredModal = useGameStore((s) => s.setShowFiredModal);
  const [isAdvancing, setIsAdvancing] = useState(false);
  const [showContinueMenu, setShowContinueMenu] = useState(false);
  const [showMatchConfirm, setShowMatchConfirm] = useState(false);
  const [matchMode, setMatchMode] = useState<MatchModeType>("live");
  const [blockerModal, setBlockerModal] = useState<BlockerModal | null>(null);
  const [recapResults, setRecapResults] = useState<
    AdvanceMatchResultData[] | null
  >(null);

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
        setGameState(result.game as GameStateData);
        if (result.results && result.results.length > 0) {
          setRecapResults(result.results);
        }
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
    const blockers = await checkBlockingActions("handleContinue");
    if (blockers.length > 0) {
      setBlockerModal({ blockers, pendingAction: () => doAdvance(resolvedMode) });
      return;
    }
    doAdvance(resolvedMode);
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

  const doSkipToMatchDay = async () => {
    console.info("[useAdvanceTime] doSkipToMatchDay:start");
    setIsAdvancing(true);
    resetTransientUi();
    try {
      const result = await skipToMatchDay();
      console.info("[useAdvanceTime] doSkipToMatchDay:result", {
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
      if (result.game) setGameState(result.game as GameStateData);
      if (result.action === "blocked" && result.blockers && result.blockers.length > 0) {
        setBlockerModal({ blockers: result.blockers });
      } else if (result.results && result.results.length > 0) {
        setRecapResults(result.results);
      }
    } catch (err) {
      console.error("Failed to skip to match day:", err);
    } finally {
      console.info("[useAdvanceTime] doSkipToMatchDay:complete");
      setIsAdvancing(false);
    }
  };

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
  };
}
