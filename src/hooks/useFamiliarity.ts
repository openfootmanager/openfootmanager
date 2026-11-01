import { useEffect, useMemo, useState } from "react";

import { getFamiliarity } from "../services/positionService";
import { useGameStore } from "../store/gameStore";
import type { Familiarity } from "../store/generated/Familiarity";
import type { FamiliarityProjection } from "../store/generated/FamiliarityProjection";
import { createSharedLoader } from "./sharedLoader";

export interface FamiliarityTable {
  /** False until the backend has answered for the club as it stands now. */
  loaded: boolean;
  /**
   * How `playerId` stands at `position` (natural, can play, out of position), or null until the
   * table is loaded, for a player outside the manager's club and for a position it does not name.
   */
  familiarityOf: (playerId: string, position: string) => Familiarity | null;
}

const loader = createSharedLoader<FamiliarityProjection>(
  // A reply that is not a projection (a screen test's stand-in) grades nobody.
  async () => {
    const answer = await getFamiliarity();
    return Array.isArray(answer?.players) ? answer : { players: [] };
  },
  "familiarity",
);

/** Forget what was loaded, so a test can start from the first request again. */
export function resetFamiliarityCache(): void {
  loader.reset();
}

/** Asks for the table now, so it is there when a screen needs it (the dashboard does at mount). */
export function prefetchFamiliarity(revision: string): () => void {
  if (revision === "") return () => {};
  return loader.subscribe(revision, () => {});
}

/**
 * What the table depends on: the career, the club, and each of its players with the positions he
 * is known for. A transfer or a position change makes a new revision and so a new request. Empty
 * when there is no club.
 */
export function useFamiliarityRevision(): string {
  const careerToken = useGameStore((state) => state.gameState?.career_token ?? 0);
  const clubId = useGameStore((state) => state.gameState?.manager.team_id ?? "");
  const players = useGameStore((state) => state.gameState?.players);
  return useMemo(() => {
    // No club (no game, or a manager without a job): the backend has nothing to project.
    if (clubId === "") return "";
    // A screen test's stand-in game may carry no player list.
    const squad = (Array.isArray(players) ? players : [])
      .filter((player) => player.team_id === clubId)
      .map(
        (player) =>
          `${player.id}:${player.natural_position}:${(player.alternate_positions ?? []).join("+")}`,
      );
    return `${careerToken}|${clubId}|${squad.join(",")}`;
  }, [careerToken, clubId, players]);
}

function tableOf(projection: FamiliarityProjection | null): FamiliarityTable {
  if (!projection) return { loaded: false, familiarityOf: () => null };
  const byPlayer = new Map(projection.players.map((row) => [row.player_id, row.by_position]));
  return {
    loaded: true,
    familiarityOf: (playerId, position) => byPlayer.get(playerId)?.[position] ?? null,
  };
}

/**
 * The backend's familiarity grade for the manager's club's players, shared by every screen that
 * reads it: one request per revision of the club, retried while a screen waits for it.
 */
export function useFamiliarity(): FamiliarityTable {
  const revision = useFamiliarityRevision();
  const [answer, setAnswer] = useState<{
    revision: string;
    projection: FamiliarityProjection;
  } | null>(() => {
    const cached = loader.cached(revision);
    return cached ? { revision, projection: cached } : null;
  });

  useEffect(() => {
    if (revision === "") return undefined;
    return loader.subscribe(revision, (projection) => setAnswer({ revision, projection }));
  }, [revision]);

  // An answer for an earlier revision is not the club as it stands: not loaded until the new one lands.
  const current = answer?.revision === revision ? answer.projection : null;
  return useMemo(() => tableOf(current), [current]);
}
