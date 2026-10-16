import { useEffect, useState } from "react";

import { fetchHallOfFameLegends } from "../../services/retireesService";
import type { HallOfFameLegendData } from "../../store/types";

/** The legends, asked of the backend again whenever the game's date moves on. */
export function useHallOfFameLegends(gameDate: string): HallOfFameLegendData[] {
  const [legends, setLegends] = useState<HallOfFameLegendData[]>([]);

  useEffect(() => {
    let cancelled = false;
    fetchHallOfFameLegends()
      .then((fetched) => {
        if (!cancelled) setLegends(fetched);
      })
      .catch(() => {
        if (!cancelled) setLegends([]);
      });
    return () => {
      cancelled = true;
    };
  }, [gameDate]);

  return legends;
}
