import { useEffect, useMemo, useState } from "react";

import { getTacticsLineup } from "../services/squadService";
import type { PlayerData, TeamData } from "../store/gameStore";
import type { TacticsLineup } from "../store/generated/TacticsLineup";
import { createSharedLoader } from "./sharedLoader";

export interface TacticsLineupAnswer {
  /** False until the backend has answered for the club as it stands now. */
  loaded: boolean;
  /**
   * The side the match would field, or null until it is loaded. Null once loaded only for a reply
   * that is not a lineup (a screen test's stand-in), which names no side.
   */
  lineup: TacticsLineup | null;
}

const loader = createSharedLoader<TacticsLineup | null>(
  // A reply that is not a lineup (a screen test's stand-in) names no side.
  async () => {
    const answer = await getTacticsLineup();
    return Array.isArray(answer?.eleven) ? answer : null;
  },
  "tactics lineup",
);

/** Forget what was loaded, so a test can start from the first request again. */
export function resetTacticsLineupCache(): void {
  loader.reset();
}

/**
 * What the side depends on: the career, the club, the day, its formation, its saved eleven and
 * roles, and each of its players' availability. Any of these changing is a new revision and so a
 * new request. Empty when there is no club.
 */
export function tacticsLineupRevision(
  careerToken: number,
  team: TeamData | null,
  players: PlayerData[],
  clockDate: string,
): string {
  if (!team) return "";
  const squad = players
    .filter((player) => player.team_id === team.id)
    .map((player) => [
      player.id,
      player.injury?.days_remaining ?? 0,
      player.squad_role ?? null,
      player.match_day_eligible ?? null,
      player.condition,
    ]);
  // JSON, not a join: an id may hold any separator a join would use.
  return JSON.stringify([
    careerToken,
    team.id,
    clockDate,
    team.formation,
    team.starting_xi_ids ?? [],
    team.player_roles ?? {},
    squad,
  ]);
}

/**
 * The backend's lineup for the manager's club (`get_tactics_lineup`): the eleven the match would
 * field, slot by slot, the bench and the rest. One request per revision, shared by every screen
 * that reads it and retried while one waits for it.
 */
export function useTacticsLineupProjection(revision: string): TacticsLineupAnswer {
  const [answer, setAnswer] = useState<{ revision: string; lineup: TacticsLineup | null } | null>(
    () => {
      const cached = loader.cached(revision);
      return cached === undefined ? null : { revision, lineup: cached };
    },
  );

  useEffect(() => {
    if (revision === "") return undefined;
    return loader.subscribe(revision, (lineup) => setAnswer({ revision, lineup }));
  }, [revision]);

  // An answer for an earlier revision is not the club as it stands: not loaded until the new one lands.
  const current = answer?.revision === revision ? answer : null;
  return useMemo(() => ({ loaded: current !== null, lineup: current?.lineup ?? null }), [current]);
}
