import { useEffect, useState } from "react";

import type { TeamData } from "../../store/gameStore";
import { getTacticsBlueprintStatus } from "../../services/squadService";

/**
 * Whether the manager's phase dials differ from their play style's blueprint. That is the backend's rule
 * (`blueprint_for`); it is asked again whenever the dials or the style change, and answered "no" for a
 * game with no club, which has no blueprint to ask about.
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
    getTacticsBlueprintStatus()
      .then(({ modified }) => {
        if (!cancelled) setBlueprintModified(modified);
      })
      .catch(() => {
        if (!cancelled) setBlueprintModified(false);
      });
    return () => {
      cancelled = true;
    };
  }, [blueprintKey, hasClub]);

  return blueprintModified;
}
