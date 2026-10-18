import { useEffect, useState } from "react";

import { type ContractWarningStage, getContractWarningStages } from "../services/contractService";
import { type ContractRiskLevel, contractRiskLevelOf } from "../lib/contractUtils";
import type { PlayerData } from "../store/gameStore";

type ContractedPlayer = Pick<PlayerData, "id" | "contract_end" | "morale_core">;

/**
 * How the contracts of these players read on a badge, from the backend's warning stages (the ones
 * the daily warnings use). It asks again when the players, their contracts or the `date` (the
 * game's) change, keeps the last answer while the next is on its way, and reads "stable" until the
 * first. A same-day renewal, expiry mark or termination moves the contracts, not the date, so the
 * contract terms are part of the cache key; the key decides when to ask, never what the stage is.
 */
export function useContractRiskLevels(
  players: ContractedPlayer[],
  date?: string,
): (playerId: string) => ContractRiskLevel {
  const [stages, setStages] = useState<Record<string, ContractWarningStage | null>>({});
  const key = players.map((player) => player.id).join(",");
  const revision = [
    date,
    ...players.map(
      (player) =>
        `${player.contract_end ?? ""}/${player.morale_core?.renewal_state?.exit_intent?.set_on ?? ""}`,
    ),
  ].join("|");

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
