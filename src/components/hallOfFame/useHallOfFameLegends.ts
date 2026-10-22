import { useBackendList } from "../../hooks/useBackendList";
import { fetchHallOfFameLegends } from "../../services/retireesService";
import type { HallOfFameLegendData } from "../../store/types";

/** The legends, asked of the backend again whenever the game's date moves on. */
export function useHallOfFameLegends(gameDate: string): HallOfFameLegendData[] {
  return useBackendList(fetchHallOfFameLegends, gameDate);
}
