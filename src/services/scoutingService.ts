import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export async function sendScout(
  scoutId: string,
  playerId: string,
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("send_scout", {
    scoutId,
    playerId,
  });
}