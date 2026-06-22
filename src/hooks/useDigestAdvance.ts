import { useState, useRef } from "react";

import type { GameStateData } from "../store/gameStore";
import type { BlockerData } from "../services/advanceTimeService";
import { advanceOneDay } from "../services/advanceTimeService";
import { buildAdvanceRecap } from "../components/dashboard/advanceRecap";
import type { AdvanceRecap } from "../components/dashboard/advanceRecap";

export interface DigestEntry {
  date: string;
  recap: AdvanceRecap;
}

export type DigestStopReason =
  | { kind: "match_day" }
  | { kind: "blocked"; blockers: BlockerData[] }
  | { kind: "fired" }
  | { kind: "stopped" }
  | { kind: "error" };

const MAX_DIGEST_DAYS = 60;

export function useDigestAdvance(
  setGameState: (state: GameStateData) => void,
  onFired: () => void,
) {
  const [isRunning, setIsRunning] = useState(false);
  const [isAborting, setIsAborting] = useState(false);
  const [entries, setEntries] = useState<DigestEntry[]>([]);
  const [stopReason, setStopReason] = useState<DigestStopReason | null>(null);
  // Abort flag so an in-flight loop can be cancelled (e.g. on unmount).
  const abortRef = useRef(false);
  const inFlightRef = useRef(false);

  const startDigest = async () => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    abortRef.current = false;
    setIsRunning(true);
    setIsAborting(false);
    setEntries([]);
    setStopReason(null);

    let daysProcessed = 0;

    try {
      while (daysProcessed < MAX_DIGEST_DAYS) {
        if (abortRef.current) {
          setStopReason({ kind: "stopped" });
          return;
        }

        const result = await advanceOneDay();

        if (abortRef.current) {
          setStopReason({ kind: "stopped" });
          return;
        }

        if (result.action === "fired") {
          if (result.game) setGameState(result.game as GameStateData);
          setStopReason({ kind: "fired" });
          onFired();
          return;
        }

        if (result.action === "match_day") {
          setStopReason({ kind: "match_day" });
          return;
        }

        if (result.action === "blocked") {
          setStopReason({ kind: "blocked", blockers: result.blockers ?? [] });
          return;
        }

        // action === "advanced"
        if (result.game) {
          const game = result.game as GameStateData;
          setGameState(game);
          const recap = buildAdvanceRecap(game, result.date, result.results ?? []);
          setEntries((prev) => [...prev, { date: result.date, recap }]);
          daysProcessed++;
        }
      }
    } catch (err) {
      console.error("[useDigestAdvance] error during digest loop:", err);
      setStopReason({ kind: "error" });
    } finally {
      inFlightRef.current = false;
      setIsRunning(false);
      setIsAborting(false);
    }
  };

  const abortDigest = () => {
    abortRef.current = true;
    setIsAborting(true);
  };

  const dismissDigest = () => {
    setEntries([]);
    setStopReason(null);
  };

  const isVisible = isRunning || entries.length > 0 || stopReason !== null;

  return {
    isRunning,
    isAborting,
    entries,
    stopReason,
    isVisible,
    startDigest,
    abortDigest,
    dismissDigest,
  };
}
