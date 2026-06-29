import { useMemo, useState } from "react";

import type { PlayerData } from "../../store/gameStore";
import {
  filterAndSortTacticsPlayers,
  type SortKey,
} from "./TacticsTab.helpers";

interface UseTacticsFiltersArgs {
  startingXI: PlayerData[];
  bench: PlayerData[];
  xiActivePosition: Map<string, string>;
}

export function useTacticsFilters({
  startingXI,
  bench,
  xiActivePosition,
}: UseTacticsFiltersArgs) {
  const [playerSearch, setPlayerSearch] = useState("");
  const [positionFilter, setPositionFilter] = useState("All");
  const sortKey: SortKey = "pos";
  const sortDir: "asc" | "desc" = "asc";

  const filteredStartingXI = useMemo(
    () =>
      filterAndSortTacticsPlayers(
        startingXI,
        {
          playerSearch,
          positionFilter,
          section: "xi",
          xiActivePosition,
        },
        {
          section: "xi",
          sortDir,
          sortKey,
          xiActivePosition,
        },
      ),
    [
      startingXI,
      playerSearch,
      positionFilter,
      sortKey,
      sortDir,
      xiActivePosition,
    ],
  );
  const filteredBench = useMemo(
    () =>
      filterAndSortTacticsPlayers(
        bench,
        {
          playerSearch,
          positionFilter,
          section: "bench",
          xiActivePosition,
        },
        {
          section: "bench",
          sortDir,
          sortKey,
          xiActivePosition,
        },
      ),
    [bench, playerSearch, positionFilter, sortKey, sortDir, xiActivePosition],
  );

  function handleClearFilters(): void {
    setPlayerSearch("");
    setPositionFilter("All");
  }

  return {
    playerSearch,
    setPlayerSearch,
    positionFilter,
    setPositionFilter,
    filteredStartingXI,
    filteredBench,
    handleClearFilters,
  };
}
