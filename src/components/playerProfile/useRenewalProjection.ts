import { useCallback, useEffect, useState } from "react";

import { previewRenewalProjection } from "../../services/contractService";
import type { RenewalProjectionData } from "./PlayerProfile.renewal";

/**
 * The club's finances if `wage` were offered to the player: asked for while `enabled` (the modal is
 * open and the wage is a number), held until the wage changes, and dropped if the answer for an
 * earlier wage arrives after it, or after the modal is shut. A failed request holds nothing.
 */
export function useRenewalProjection(playerId: string, wage: number, enabled: boolean) {
  const [projection, setProjection] = useState<RenewalProjectionData["projection"] | null>(null);

  useEffect(() => {
    if (!enabled) {
      setProjection(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await previewRenewalProjection(playerId, wage);

        if (!cancelled) {
          setProjection(result.projection ?? null);
        }
      } catch {
        if (!cancelled) {
          setProjection(null);
        }
      }
    };

    void loadProjection();

    return () => {
      cancelled = true;
    };
  }, [enabled, playerId, wage]);

  const clearProjection = useCallback(() => {
    setProjection(null);
  }, []);

  return { projection, clearProjection };
}
