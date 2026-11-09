import { useEffect, useState } from "react";
import { getFollowedDivision } from "../../services/seasonReviewService";
import type { GameStateData } from "../../store/gameStore";

type Result =
  | { game: GameStateData; attempt: number; league: GameStateData["league"] }
  | { game: GameStateData; attempt: number; error: true };

export function useReviewDivision(game: GameStateData, enabled: boolean) {
  const [result, setResult] = useState<Result | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let active = true;
    if (enabled) {
      getFollowedDivision(game.career_token ?? 0).then(
        (league) => {
          if (active) setResult({ game, attempt, league });
        },
        (error: unknown) => {
          console.error("Failed to load followed division:", error);
          if (active) setResult({ game, attempt, error: true });
        },
      );
    }
    return () => {
      active = false;
    };
  }, [game, enabled, attempt]);
  const current = enabled && result?.game === game && result.attempt === attempt ? result : null;
  return {
    league: current && "league" in current ? current.league : null,
    loading: enabled && !current,
    failed: !!current && "error" in current,
    retry: () => setAttempt((n) => n + 1),
  };
}
