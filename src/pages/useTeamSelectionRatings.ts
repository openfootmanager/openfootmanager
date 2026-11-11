import { useEffect, useMemo, useState } from "react";

import type { GameStateData } from "../store/gameStore";
import {
  type ClubSquadRatings,
  fetchTeamSelectionRatings,
  type TeamSelectionRatings,
} from "../services/teamsService";

/**
 * Every club's squad as the backend rates it before a career (`get_team_selection_ratings`): the
 * team-selection screen shows what the backend decides, not ratings read off the game state.
 * Asked again whenever the game changes; null figures until it answers.
 */
export function useTeamSelectionRatings(gameState: GameStateData | null) {
  const [ratings, setRatings] = useState<TeamSelectionRatings | null>(null);

  useEffect(() => {
    if (!gameState) return;
    let cancelled = false;
    void (async () => {
      try {
        const answer = await fetchTeamSelectionRatings();
        if (!cancelled) setRatings(answer ?? null);
      } catch (error: unknown) {
        console.error("Failed to fetch the squads' ratings", error);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [gameState]);

  const ratingsByClub = useMemo(
    () => new Map((ratings?.clubs ?? []).map((club) => [club.team_id, club])),
    [ratings],
  );
  const ratingByPlayer = useMemo(
    () =>
      new Map(
        (ratings?.clubs ?? []).flatMap((club) =>
          club.players.map((rated) => [rated.player_id, rated.ovr] as const),
        ),
      ),
    [ratings],
  );

  return {
    /** A club's rated squad, for its likely eleven; undefined until the backend answers. */
    clubRatings: (teamId: string): ClubSquadRatings | undefined => ratingsByClub.get(teamId),
    /** The club's average as the backend rates it; null until it answers. */
    getTeamAvgOvr: (teamId: string): number | null =>
      ratingsByClub.get(teamId)?.average_rating ?? null,
    /** A player's rating as the backend rates it; null until it answers. */
    getPlayerRating: (playerId: string): number | null => ratingByPlayer.get(playerId) ?? null,
  };
}
