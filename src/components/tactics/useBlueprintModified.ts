import { useEffect, useState } from "react";

import type { TeamData } from "../../store/gameStore";
import { getTacticsBlueprintStatus } from "../../services/squadService";

/** How long to wait before asking again after a failure: three retries, then it stops. */
const RETRY_DELAYS_MS = [1_000, 3_000, 9_000];

/**
 * Whether the manager's phase dials differ from their play style's blueprint. That is the backend's rule
 * (`blueprint_for`); it is asked again whenever the dials or the style change, and answered "no" for a
 * game with no club, which has no blueprint to ask about.
 *
 * A question the backend cannot answer reads as "differs", not as "no": a failed check must not hide
 * Reset on a tuned preset (the same call as for a saved tactic's differences). It is asked again after
 * a short wait, three times, so a transient refusal does not leave the guess standing.
 */
export function useBlueprintModified(
  activePlayStyle: string,
  userTeam: TeamData | undefined,
): boolean {
  const [blueprintModified, setBlueprintModified] = useState(false);
  const blueprintKey = `${activePlayStyle}|${JSON.stringify(userTeam?.tactics_phase ?? null)}`;

  const hasClub = Boolean(userTeam);

  useEffect(() => {
    // No game or no club has no blueprint to ask about.
    if (!hasClub) {
      setBlueprintModified(false);
      return;
    }

    let cancelled = false;
    let retry: ReturnType<typeof setTimeout> | undefined;
    let failures = 0;
    const ask = () => {
      getTacticsBlueprintStatus()
        .then(({ modified }) => {
          if (!cancelled) setBlueprintModified(modified);
        })
        .catch(() => {
          if (cancelled) return;
          setBlueprintModified(true);
          const delay = RETRY_DELAYS_MS[failures];
          failures += 1;
          if (delay !== undefined) retry = setTimeout(ask, delay);
        });
    };
    ask();
    return () => {
      cancelled = true;
      if (retry) clearTimeout(retry);
    };
  }, [blueprintKey, hasClub]);

  return blueprintModified;
}
