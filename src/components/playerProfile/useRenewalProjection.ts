import { useCallback, useEffect, useState } from "react";

import { previewRenewalProjection } from "../../services/contractService";
import type { RenewalProjectionData } from "./PlayerProfile.renewal";

/** A projection and the request it answers, so a stale answer cannot stand for the current wage. */
interface HeldProjection {
  playerId: string;
  wage: number;
  projection: RenewalProjectionData["projection"] | null;
}

/**
 * The club's finances if `wage` were offered to the player: asked for while `enabled` (the modal is
 * open and the wage is a number), held until the wage changes, and dropped if the answer for an
 * earlier wage arrives after it, or after the modal is shut. A failed request holds nothing.
 *
 * The projection is only returned while it answers the wage on screen; while the next request is
 * loading, the previous wage's answer would otherwise show a restriction that no longer applies.
 */
export function useRenewalProjection(playerId: string, wage: number, enabled: boolean) {
  const [held, setHeld] = useState<HeldProjection | null>(null);

  useEffect(() => {
    if (!enabled) {
      setHeld(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await previewRenewalProjection(playerId, wage);

        if (!cancelled) {
          setHeld({ playerId, wage, projection: result.projection ?? null });
        }
      } catch {
        if (!cancelled) {
          setHeld({ playerId, wage, projection: null });
        }
      }
    };

    void loadProjection();

    return () => {
      cancelled = true;
    };
  }, [enabled, playerId, wage]);

  const projection =
    enabled && held !== null && held.playerId === playerId && held.wage === wage
      ? held.projection
      : null;

  const clearProjection = useCallback(() => {
    setHeld(null);
  }, []);

  return { projection, clearProjection };
}
