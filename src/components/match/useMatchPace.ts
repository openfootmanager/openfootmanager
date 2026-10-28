import { useCallback, useState } from "react";

import type { MatchPace } from "./MatchSpeedControl";
import { isPersistableSpeed } from "./types";

/**
 * Whether the live match is playing and at what pace. Pausing keeps the pace, so Resume plays at
 * the speed the player left, and only a pace worth keeping (not Max) is reported for the second
 * half.
 */
export function useMatchPace(
  initialPace: MatchPace,
  onPreferredSpeedChange?: (speed: "slow" | "normal" | "fast") => void,
) {
  const [pace, setPace] = useState<MatchPace>(initialPace);
  const [isRunning, setIsRunning] = useState(true);

  const pause = useCallback(() => setIsRunning(false), []);
  const togglePause = useCallback(() => setIsRunning((running) => !running), []);
  const selectPace = useCallback(
    (next: MatchPace) => {
      setPace(next);
      setIsRunning(true);
      if (isPersistableSpeed(next)) onPreferredSpeedChange?.(next);
    },
    [onPreferredSpeedChange],
  );

  return { pace, isRunning, pause, togglePause, selectPace };
}
