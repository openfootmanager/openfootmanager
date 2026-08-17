import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import {
  buildPlayerAdvancedStats,
  type PlayerAdvancedStatsSummary,
} from "./PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "./PlayerProfileRecentMatchesCard";

const RECENT_MATCH_LIMIT = 5;

interface UsePlayerProfileDataArgs {
  player: PlayerData;
  gameState: GameStateData;
}

interface UsePlayerProfileDataResult {
  /** The backend's figures when they differ from what the squad data implies. */
  advancedStats: PlayerAdvancedStatsSummary;
  recentMatches: PlayerRecentMatchEntry[];
}

function areAdvancedStatsEqual(
  left: PlayerAdvancedStatsSummary,
  right: PlayerAdvancedStatsSummary,
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function haveSameFixtures(
  left: PlayerRecentMatchEntry[],
  right: PlayerRecentMatchEntry[],
): boolean {
  return (
    left.length === right.length &&
    left.every((entry, index) => entry.fixture_id === right[index]?.fixture_id)
  );
}

/**
 * The two things a profile fetches that the game state does not already hold:
 * the backend's own advanced-stats view, and the player's last few matches.
 *
 * Both render from squad data first and upgrade in place, so the profile is
 * never blank while a request is in flight. Both also keep the previous object
 * identity when the new payload is equivalent — these feed a chart and a list
 * that would otherwise re-render on every poll.
 */
export function usePlayerProfileData({
  player,
  gameState,
}: UsePlayerProfileDataArgs): UsePlayerProfileDataResult {
  const [advancedStatsOverride, setAdvancedStatsOverride] =
    useState<PlayerAdvancedStatsSummary | null>(null);
  const [recentMatches, setRecentMatches] = useState<PlayerRecentMatchEntry[]>(
    [],
  );

  const fallbackAdvancedStats = buildPlayerAdvancedStats(
    player,
    gameState.players,
  );

  useEffect(() => {
    let cancelled = false;

    setAdvancedStatsOverride((current) => (current === null ? current : null));

    const loadAdvancedStats = async (): Promise<void> => {
      try {
        const result = await invoke<PlayerAdvancedStatsSummary>(
          "get_player_stats_overview",
          {
            playerId: player.id,
          },
        );

        if (
          !cancelled &&
          !areAdvancedStatsEqual(result, fallbackAdvancedStats)
        ) {
          setAdvancedStatsOverride(result);
        }
      } catch {
        if (!cancelled) {
          setAdvancedStatsOverride((current) =>
            current === null ? current : null,
          );
        }
      }
    };

    void loadAdvancedStats();

    return () => {
      cancelled = true;
    };
    // Deliberately keyed on the raw stat counters rather than on
    // `fallbackAdvancedStats`, which is rebuilt every render and would refetch
    // on each one.
  }, [
    player.id,
    player.stats.minutes_played,
    player.stats.shots,
    player.stats.shots_on_target,
    player.stats.passes_completed,
    player.stats.passes_attempted,
    player.stats.tackles_won,
    player.stats.interceptions,
    player.stats.fouls_committed,
  ]);

  useEffect(() => {
    // Clear first, the way the advanced-stats effect above does. The profile is
    // not remounted when the manager moves to another player, so holding the
    // old list until the new one arrives shows it under the wrong name.
    setRecentMatches((current) => (current.length === 0 ? current : []));

    if (player.stats.appearances <= 0) {
      return;
    }

    let cancelled = false;

    const loadRecentMatches = async (): Promise<void> => {
      try {
        const result = await invoke<PlayerRecentMatchEntry[]>(
          "get_player_match_history",
          {
            playerId: player.id,
            limit: RECENT_MATCH_LIMIT,
          },
        );

        if (!cancelled) {
          setRecentMatches((current) =>
            haveSameFixtures(current, result) ? current : result,
          );
        }
      } catch {
        if (!cancelled) {
          setRecentMatches((current) => (current.length === 0 ? current : []));
        }
      }
    };

    void loadRecentMatches();

    return () => {
      cancelled = true;
    };
  }, [player.id, player.stats.appearances]);

  return {
    advancedStats: advancedStatsOverride ?? fallbackAdvancedStats,
    recentMatches,
  };
}
