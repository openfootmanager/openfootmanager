import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import type { PlayerData } from "../../store/gameStore";
import { buildPlayerAdvancedStats, type PlayerAdvancedStatsSummary } from "./PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "./PlayerProfile.helpers";

const RECENT_MATCH_LIMIT = 5;

interface UsePlayerProfileDataArgs {
  player: PlayerData;
  /** The game date: the league peer group changes with a transfer or a split season's second half. */
  currentDate: string;
}

interface UsePlayerProfileDataResult {
  /** The backend's figures when they differ from what the squad data implies. */
  advancedStats: PlayerAdvancedStatsSummary;
  recentMatches: PlayerRecentMatchEntry[];
  /**
   * Where the history stands: not answered yet, answered (and the list may be empty: no matches
   * played), or failed. A failed or pending history is never "no matches".
   */
  recentMatchesStatus: "loading" | "answered" | "failed";
  /** Asks for the history again after a failure. */
  retryRecentMatches: () => void;
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
  currentDate,
}: UsePlayerProfileDataArgs): UsePlayerProfileDataResult {
  const [advancedStatsOverride, setAdvancedStatsOverride] =
    useState<PlayerAdvancedStatsSummary | null>(null);
  const [recentMatches, setRecentMatches] = useState<PlayerRecentMatchEntry[]>([]);
  const [recentMatchesStatus, setRecentMatchesStatus] =
    useState<UsePlayerProfileDataResult["recentMatchesStatus"]>("loading");
  const [historyAttempt, setHistoryAttempt] = useState(0);
  const listedPlayerId = useRef<string | null>(null);
  // Whether the list on screen holds any match: a refresh that fails keeps one that does, and
  // reports the failure when the only answer so far was an empty list.
  const listHoldsMatches = useRef(false);

  const fallbackAdvancedStats = buildPlayerAdvancedStats(player);

  useEffect(() => {
    let cancelled = false;

    setAdvancedStatsOverride((current) => (current === null ? current : null));

    const loadAdvancedStats = async (): Promise<void> => {
      try {
        const result = await invoke<PlayerAdvancedStatsSummary>("get_player_stats_overview", {
          playerId: player.id,
        });

        if (!cancelled && !areAdvancedStatsEqual(result, fallbackAdvancedStats)) {
          setAdvancedStatsOverride(result);
        }
      } catch {
        if (!cancelled) {
          setAdvancedStatsOverride((current) => (current === null ? current : null));
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
    player.team_id,
    currentDate,
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
    // Clear when the player changes, the way the advanced-stats effect above does. The profile
    // is not remounted when the manager moves to another player, so holding the old list until
    // the new one arrives shows it under the wrong name. A refresh for the same player (the date
    // moved) keeps what is shown until the new list arrives, and when it fails.
    if (listedPlayerId.current !== player.id) {
      listedPlayerId.current = player.id;
      setRecentMatches((current) => (current.length === 0 ? current : []));
      listHoldsMatches.current = false;
      setRecentMatchesStatus("loading");
    }

    let cancelled = false;

    const loadRecentMatches = async (): Promise<void> => {
      try {
        const result = await invoke<PlayerRecentMatchEntry[]>("get_player_match_history", {
          playerId: player.id,
          limit: RECENT_MATCH_LIMIT,
        });

        if (!cancelled) {
          setRecentMatches((current) => (haveSameFixtures(current, result) ? current : result));
          listHoldsMatches.current = result.length > 0;
          setRecentMatchesStatus("answered");
        }
      } catch {
        // The list already on screen is this player's (a change of player cleared it above), and
        // a failed refresh is not an empty history: leave it, answered, when it holds matches.
        // A history that never answered, or answered with none, is failed, never "no matches".
        if (!cancelled) {
          setRecentMatchesStatus((current) =>
            current === "answered" && listHoldsMatches.current ? current : "failed",
          );
        }
      }
    };

    void loadRecentMatches();

    return () => {
      cancelled = true;
    };
  }, [player.id, player.stats.appearances, currentDate, historyAttempt]);

  return {
    advancedStats: advancedStatsOverride ?? fallbackAdvancedStats,
    recentMatches,
    recentMatchesStatus,
    retryRecentMatches: () => {
      setRecentMatchesStatus("loading");
      setHistoryAttempt((count) => count + 1);
    },
  };
}
