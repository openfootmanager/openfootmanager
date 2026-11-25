import { useCallback, useEffect, useState } from "react";

import { previewRenewalProjection } from "../../services/contractService";
import type { RenewalProjectionData } from "./PlayerProfile.renewal";

/**
 * The club's finances if `wage` were offered to the player: asked for while `enabled` (the modal is
 * open and the wage is a number). Only a response for the current player and wage is displayed.
 * Responses that arrive after their request is superseded or the modal closes are discarded.
 */
export function useRenewalProjection(playerId: string, wage: number, enabled: boolean) {
  const [response, setResponse] = useState<{
    playerId: string;
    wage: number;
    projection: RenewalProjectionData["projection"];
  } | null>(null);

  useEffect(() => {
    setResponse(null);
    if (!enabled) {
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      const result = await previewRenewalProjection(playerId, wage).catch(() => null);
      if (!cancelled && result?.projection) {
        setResponse({ playerId, wage, projection: result.projection });
      }
    };

    void loadProjection();

    return () => {
      cancelled = true;
    };
  }, [enabled, playerId, wage]);

  const clearProjection = useCallback(() => {
    setResponse(null);
  }, []);

  const projection =
    enabled && response?.playerId === playerId && response.wage === wage
      ? response.projection
      : null;
  return { projection, clearProjection };
}
