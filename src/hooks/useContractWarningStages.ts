import { useEffect, useState } from "react";

import { type ContractWarningStage, getContractWarningStages } from "../services/contractService";
import { type ContractRiskLevel, contractRiskLevelOf } from "../lib/contractUtils";

/**
 * How the contracts of these players read on a badge, from the backend's warning stages (the ones
 * the daily warnings use). It asks again when the players or the `revision` (the game's date, say)
 * change, keeps the last answer while the next is on its way, and reads "stable" until the first.
 */
export function useContractRiskLevels(
  playerIds: string[],
  revision?: unknown,
): (playerId: string) => ContractRiskLevel {
  const [stages, setStages] = useState<Record<string, ContractWarningStage | null>>({});
  const key = playerIds.join(",");

  useEffect(() => {
    if (key === "") return;
    let cancelled = false;
    getContractWarningStages(key.split(","))
      .then((answer) => {
        if (!cancelled) setStages(answer);
      })
      .catch((error: unknown) => {
        console.error("Failed to read the contract warning stages:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [key, revision]);

  return (playerId) => contractRiskLevelOf(stages[playerId]);
}
