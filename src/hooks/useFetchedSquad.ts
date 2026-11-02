import { useEffect, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";

import { getSquad } from "../services/squadService";
import type { PlayerData } from "../store/gameStore";

/** How long to wait before asking again after the nth failure in a row; the last delay repeats. */
const RETRY_DELAYS_MS = [1_000, 2_000, 5_000, 15_000];

/**
 * Fetches a team's squad and refetches whenever the team OR the game clock
 * or supplied game revision changes, so eligibility and per-day fields (condition, fitness, injuries) refresh after a day
 * is advanced — not only when the user switches tabs. A `cancelled` guard drops
 * out-of-order responses.
 *
 * Returns a `[squad, setSquad, answered]` tuple. `setSquad` works like `useState`'s so callers can
 * also patch the cached squad optimistically after a mutation. `squad` is `null` until the first
 * fetch resolves, and is kept while a newer fetch is in flight, so it can be one revision old.
 * `answered` is true only once a fetch has resolved for the team, clock and revision now asked
 * about: an action that saves a lineup worked out from the squad must wait for it, because the
 * cached squad's verdicts (eligibility, injuries) are the previous revision's until then, and an
 * optimistic patch carries them over rather than renewing them. A fetch that fails is asked again
 * with a growing delay while the caller stays mounted (a screen waiting for `answered` would
 * otherwise stay locked until the next revision), and the failure count starts over for a new
 * question.
 */
export function useFetchedSquad(
  teamId: string | null,
  clockDate: string,
  revision?: unknown,
): [PlayerData[] | null, Dispatch<SetStateAction<PlayerData[] | null>>, boolean] {
  const [fetchedSquad, setFetchedSquad] = useState<PlayerData[] | null>(null);
  // The team the cached squad belongs to. Used to scope the cache so switching
  // clubs (or going unemployed) never exposes the previous team's roster while
  // the new fetch is still pending.
  const [fetchedTeamId, setFetchedTeamId] = useState<string | null>(null);
  // What the last resolved fetch was asked about; the squad is current only if that is what is
  // asked about now.
  // Bumped to ask the same question again after a failure.
  const [retry, setRetry] = useState(0);
  const failures = useRef(0);
  const asked = useRef<{ teamId: string | null; clockDate: string; revision: unknown } | null>(
    null,
  );
  const [answeredFor, setAnsweredFor] = useState<{
    teamId: string;
    clockDate: string;
    revision: unknown;
  } | null>(null);

  const setSquadForCurrentTeam: Dispatch<SetStateAction<PlayerData[] | null>> = (next) => {
    // With no active team there is nothing to cache — never let an optimistic
    // update repopulate (and thus re-expose) a roster in the teamless state.
    if (!teamId) {
      setFetchedTeamId(null);
      setFetchedSquad(null);
      setAnsweredFor(null);
      return;
    }
    setFetchedTeamId(teamId);
    setFetchedSquad((previous) => {
      const updated = typeof next === "function" ? next(previous) : next;
      // Full Game mutation responses omit projection metadata. Preserve it until
      // the revision-triggered fetch supplies the new backend verdict.
      const previousById = new Map(previous?.map((player) => [player.id, player]));
      return (
        updated?.map((player) => ({
          ...player,
          match_day_eligible:
            player.match_day_eligible ?? previousById.get(player.id)?.match_day_eligible,
          condition_status:
            player.condition_status ?? previousById.get(player.id)?.condition_status,
        })) ?? null
      );
    });
  };

  useEffect(() => {
    if (!teamId) {
      setFetchedTeamId(null);
      setFetchedSquad(null);
      // The squad is gone with the team; an answer recorded for it must not outlive it.
      setAnsweredFor(null);
      return;
    }
    // A new team, clock or revision is a new question: its first failure is a first failure.
    const previous = asked.current;
    if (
      !previous ||
      previous.teamId !== teamId ||
      previous.clockDate !== clockDate ||
      !Object.is(previous.revision, revision)
    ) {
      failures.current = 0;
    }
    asked.current = { teamId, clockDate, revision };
    let cancelled = false;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;
    void getSquad(teamId)
      .then((squad) => {
        if (!cancelled) {
          failures.current = 0;
          setFetchedTeamId(teamId);
          // An answer that is not a squad (a stand-in's, a broken reply) is no squad: callers fall
          // back to the game's players and nothing treats it as a roster.
          setFetchedSquad(Array.isArray(squad) ? squad : null);
          setAnsweredFor({ teamId, clockDate, revision });
        }
      })
      .catch((error: unknown) => {
        console.error("Failed to fetch squad", error);
        if (cancelled) return;
        failures.current += 1;
        const delay = RETRY_DELAYS_MS[Math.min(failures.current - 1, RETRY_DELAYS_MS.length - 1)];
        retryTimer = setTimeout(() => setRetry((count) => count + 1), delay);
      });
    return () => {
      cancelled = true;
      clearTimeout(retryTimer);
    };
  }, [teamId, clockDate, revision, retry]);

  // Only serve the cache for the team currently requested.
  const answered =
    teamId !== null &&
    answeredFor !== null &&
    answeredFor.teamId === teamId &&
    // The clock only moves with a new game state, so the revision comparison already covers it;
    // it stays so the hook's contract does not depend on what the caller passes as the revision.
    answeredFor.clockDate === clockDate &&
    Object.is(answeredFor.revision, revision);
  return [fetchedTeamId === teamId ? fetchedSquad : null, setSquadForCurrentTeam, answered];
}
