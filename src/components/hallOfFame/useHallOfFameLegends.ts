import { useBackendList } from "../../hooks/useBackendList";
import { fetchHallOfFameLegends } from "../../services/retireesService";
import type { HallOfFameLegendData } from "../../store/types";

/** The legends, asked of the backend again whenever the game's date moves on or another career is installed. */
export function useHallOfFameLegends(
  gameDate: string,
  careerToken: number,
): HallOfFameLegendData[] {
  return useBackendList(fetchHallOfFameLegends, `${gameDate}|${careerToken}`);
}
