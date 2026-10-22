import { useBackendList } from "../../hooks/useBackendList";
import { fetchHallOfFameChampions } from "../../services/retireesService";
import type { PastChampionData } from "../../store/types";

/** The league titles of the world's past, asked of the backend again whenever the date moves on. */
export function useHallOfFameChampions(gameDate: string): PastChampionData[] {
  return useBackendList(fetchHallOfFameChampions, gameDate);
}
