import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export type FacilityId = "Training" | "Medical" | "Scouting";

export async function upgradeFacility(
    facility: FacilityId,
): Promise<GameStateData> {
    return invokeCommand<GameStateData>("upgrade_facility", {
        facility,
    });
}