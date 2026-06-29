import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import type { PlayerData } from "../../store/gameStore";
import {
  buildPlayerAdvancedStats,
  type PlayerAdvancedStatsSummary,
} from "./PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "./PlayerProfileRecentMatchesCard";

interface UsePlayerProfileDataArgs {
  player: PlayerData;
  allPlayers: PlayerData[];
}

interface UsePlayerProfileDataResult {
  advancedStats: PlayerAdvancedStatsSummary;
  recentMatches: PlayerRecentMatchEntry[];
}

function areAdvancedStatsEqual(
  left: PlayerAdvancedStatsSummary,
  right: PlayerAdvancedStatsSummary,
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function usePlayerProfileData({
  player,
  allPlayers,
}: UsePlayerProfileDataArgs): UsePlayerProfileDataResult {
  const [advancedStatsOverride, setAdvancedStatsOverride] =
    useState<PlayerAdvancedStatsSummary | null>(null);
  const [recentMatches, setRecentMatches] = useState<PlayerRecentMatchEntry[]>(
    [],
  );

  const fallbackAdvancedStats = buildPlayerAdvancedStats(player, allPlayers);
  const advancedStats = advancedStatsOverride ?? fallbackAdvancedStats;

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

        if (!cancelled && !areAdvancedStatsEqual(result, fallbackAdvancedStats)) {
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
    if (player.stats.appearances <= 0) {
      setRecentMatches([]);
      return;
    }

    let cancelled = false;

    const loadRecentMatches = async (): Promise<void> => {
      try {
        const result = await invoke<PlayerRecentMatchEntry[]>(
          "get_player_match_history",
          {
            playerId: player.id,
            limit: 5,
          },
        );

        if (!cancelled) {
          setRecentMatches((current) => {
            if (
              current.length === result.length &&
              current.every(
                (entry, index) => entry.fixture_id === result[index]?.fixture_id,
              )
            ) {
              return current;
            }

            return result;
          });
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

  return { advancedStats, recentMatches };
}
